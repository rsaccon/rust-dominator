use discard::DiscardOnDrop;
use futures_core::task::{LocalWaker, Waker};
use futures_core::Poll;
use futures_signals::signal::{Mutable, MutableSignal, Signal, SignalExt, WaitFor};
use futures_signals::signal_vec::{SignalVec, VecDiff};
use futures_signals::CancelableFutureHandle;
use futures_util::future::{ready, FutureExt};
use operations::spawn_future;
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use std::fmt;
use std::marker::Unpin;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock, Weak};
use stdweb::Value;

// TODO generalize this so it works for any target, not just JS
struct Raf(Value);

impl Raf {
    #[inline]
    fn new<F>(callback: F) -> Self
    where
        F: FnMut(f64) + 'static,
    {
        Raf(js!(
            var callback = @{callback};

            function loop(time) {
                value.id = requestAnimationFrame(loop);
                callback(time);
            }

            var value = {
                callback: callback,
                id: requestAnimationFrame(loop)
            };

            return value;
        ))
    }
}

impl Drop for Raf {
    #[inline]
    fn drop(&mut self) {
        js! { @(no_return)
            var self = @{&self.0};
            cancelAnimationFrame(self.id);
            self.callback.drop();
        }
    }
}

struct TimestampsInner {
    raf: Option<Raf>,
    // TODO make this more efficient
    states: Vec<Weak<Mutex<TimestampsState>>>,
}

struct TimestampsGlobal {
    inner: Mutex<TimestampsInner>,
    value: Arc<RwLock<Option<f64>>>,
}

#[derive(Debug)]
enum TimestampsEnum {
    First,
    Changed,
    NotChanged,
}

#[derive(Debug)]
struct TimestampsState {
    state: TimestampsEnum,
    waker: Option<Waker>,
}

#[derive(Debug)]
pub struct Timestamps {
    state: Arc<Mutex<TimestampsState>>,
    // TODO verify that there aren't any Arc cycles
    value: Arc<RwLock<Option<f64>>>,
}

impl Signal for Timestamps {
    type Item = Option<f64>;

    // TODO implement Poll::Ready(None)
    fn poll_change(self: Pin<&mut Self>, waker: &LocalWaker) -> Poll<Option<Self::Item>> {
        let mut lock = self.state.lock().unwrap();

        match lock.state {
            TimestampsEnum::Changed => {
                lock.state = TimestampsEnum::NotChanged;
                Poll::Ready(Some(*self.value.read().unwrap()))
            }
            TimestampsEnum::First => {
                lock.state = TimestampsEnum::NotChanged;
                Poll::Ready(Some(None))
            }
            TimestampsEnum::NotChanged => {
                lock.waker = Some(waker.clone().into_waker());
                Poll::Pending
            }
        }
    }
}

lazy_static! {
    static ref TIMESTAMPS: Arc<TimestampsGlobal> = Arc::new(TimestampsGlobal {
        inner: Mutex::new(TimestampsInner {
            raf: None,
            states: vec![],
        }),
        value: Arc::new(RwLock::new(None)),
    });
}

pub fn timestamps() -> Timestamps {
    let timestamps = Timestamps {
        state: Arc::new(Mutex::new(TimestampsState {
            state: TimestampsEnum::First,
            waker: None,
        })),
        value: TIMESTAMPS.value.clone(),
    };

    {
        let mut lock = TIMESTAMPS.inner.lock().unwrap();

        lock.states.push(Arc::downgrade(&timestamps.state));

        if let None = lock.raf {
            let global = TIMESTAMPS.clone();

            lock.raf = Some(Raf::new(move |time| {
                let mut lock = global.inner.lock().unwrap();
                let mut value = global.value.write().unwrap();

                *value = Some(time);

                lock.states.retain(|state| {
                    if let Some(state) = state.upgrade() {
                        let mut lock = state.lock().unwrap();

                        lock.state = TimestampsEnum::Changed;

                        if let Some(waker) = lock.waker.take() {
                            drop(lock);
                            waker.wake();
                        }

                        true
                    } else {
                        false
                    }
                });

                if lock.states.len() == 0 {
                    lock.raf = None;
                    // TODO is this a good idea ?
                    lock.states = vec![];
                }
            }));
        }
    }

    timestamps
}

pub trait AnimatedSignalVec: SignalVec {
    type Animation;

    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
    where
        F: FnMut(Self::Item, Self::Animation) -> A,
        Self: Sized;
}

impl<S: SignalVec> AnimatedSignalVec for S {
    type Animation = AnimatedMapBroadcaster;

    #[inline]
    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
    where
        F: FnMut(Self::Item, Self::Animation) -> A,
    {
        AnimatedMap {
            duration: duration,
            animations: vec![],
            signal: Some(self),
            callback: f,
        }
    }
}

#[derive(Debug)]
pub struct AnimatedMapBroadcaster(MutableAnimation);

impl AnimatedMapBroadcaster {
    // TODO it should return a custom type
    #[inline]
    pub fn signal(&self) -> MutableAnimationSignal {
        self.0.signal()
    }
}

#[derive(Debug)]
struct AnimatedMapState {
    animation: MutableAnimation,
    removing: Option<WaitFor<MutableAnimationSignal>>,
}

// TODO move this into signals crate and also generalize it to work with any future, not just animations
#[derive(Debug)]
pub struct AnimatedMap<A, B> {
    duration: f64,
    animations: Vec<AnimatedMapState>,
    signal: Option<A>,
    callback: B,
}

impl<A, F, S> AnimatedMap<S, F>
where
    S: SignalVec,
    F: FnMut(S::Item, AnimatedMapBroadcaster) -> A,
{
    unsafe_unpinned!(animations: Vec<AnimatedMapState>);
    unsafe_pinned!(signal: Option<S>);
    unsafe_unpinned!(callback: F);

    fn animated_state(&self) -> AnimatedMapState {
        let state = AnimatedMapState {
            animation: MutableAnimation::new(self.duration),
            removing: None,
        };

        state.animation.animate_to(Percentage::new_unchecked(1.0));

        state
    }

    fn remove_index(self: &mut Pin<&mut Self>, index: usize) -> Poll<Option<VecDiff<A>>> {
        if index == (self.animations.len() - 1) {
            self.as_mut().animations().pop();
            Poll::Ready(Some(VecDiff::Pop {}))
        } else {
            self.as_mut().animations().remove(index);
            Poll::Ready(Some(VecDiff::RemoveAt { index }))
        }
    }

    fn should_remove(self: &mut Pin<&mut Self>, waker: &LocalWaker, index: usize) -> bool {
        let state = &mut self.as_mut().animations()[index];

        state.animation.animate_to(Percentage::new_unchecked(0.0));

        let mut future = state
            .animation
            .signal()
            .wait_for(Percentage::new_unchecked(0.0));

        if future.poll_unpin(waker).is_ready() {
            true
        } else {
            state.removing = Some(future);
            false
        }
    }

    fn find_index(&self, parent_index: usize) -> Option<usize> {
        let mut seen = 0;

        // TODO is there a combinator that can simplify this ?
        self.animations.iter().position(|state| {
            if state.removing.is_none() {
                if seen == parent_index {
                    true
                } else {
                    seen += 1;
                    false
                }
            } else {
                false
            }
        })
    }

    #[inline]
    fn find_last_index(&self) -> Option<usize> {
        self.animations
            .iter()
            .rposition(|state| state.removing.is_none())
    }
}

impl<A, B> Unpin for AnimatedMap<A, B> where A: Unpin {}

impl<A, F, S> SignalVec for AnimatedMap<S, F>
where
    S: SignalVec,
    F: FnMut(S::Item, AnimatedMapBroadcaster) -> A,
{
    type Item = A;

    // TODO this can probably be implemented more efficiently
    fn poll_vec_change(
        mut self: Pin<&mut Self>,
        waker: &LocalWaker,
    ) -> Poll<Option<VecDiff<Self::Item>>> {
        let mut is_done = true;

        // TODO is this loop correct ?
        while let Some(result) = self
            .as_mut()
            .signal()
            .as_pin_mut()
            .map(|signal| signal.poll_vec_change(waker))
        {
            match result {
                Poll::Ready(Some(change)) => {
                    return match change {
                        // TODO maybe it should play remove / insert animations for this ?
                        VecDiff::Replace { values } => {
                            *self.as_mut().animations() = Vec::with_capacity(values.len());

                            Poll::Ready(Some(VecDiff::Replace {
                                values: values
                                    .into_iter()
                                    .map(|value| {
                                        let state = AnimatedMapState {
                                            animation: MutableAnimation::new_with_initial(
                                                self.duration,
                                                Percentage::new_unchecked(1.0),
                                            ),
                                            removing: None,
                                        };

                                        let value = self.as_mut().callback()(
                                            value,
                                            AnimatedMapBroadcaster(state.animation.raw_clone()),
                                        );

                                        self.as_mut().animations().push(state);

                                        value
                                    })
                                    .collect(),
                            }))
                        }

                        VecDiff::InsertAt { index, value } => {
                            let index = self
                                .find_index(index)
                                .unwrap_or_else(|| self.animations.len());
                            let state = self.animated_state();
                            let value = self.as_mut().callback()(
                                value,
                                AnimatedMapBroadcaster(state.animation.raw_clone()),
                            );
                            self.as_mut().animations().insert(index, state);
                            Poll::Ready(Some(VecDiff::InsertAt { index, value }))
                        }

                        VecDiff::Push { value } => {
                            let state = self.animated_state();
                            let value = self.as_mut().callback()(
                                value,
                                AnimatedMapBroadcaster(state.animation.raw_clone()),
                            );
                            self.as_mut().animations().push(state);
                            Poll::Ready(Some(VecDiff::Push { value }))
                        }

                        VecDiff::UpdateAt { index, value } => {
                            let index = self.find_index(index).expect("Could not find value");
                            let state = {
                                let state = &self.as_mut().animations()[index];
                                AnimatedMapBroadcaster(state.animation.raw_clone())
                            };
                            let value = self.as_mut().callback()(value, state);
                            Poll::Ready(Some(VecDiff::UpdateAt { index, value }))
                        }

                        // TODO test this
                        // TODO should this be treated as a removal + insertion ?
                        VecDiff::Move {
                            old_index,
                            new_index,
                        } => {
                            let old_index =
                                self.find_index(old_index).expect("Could not find value");

                            let state = self.as_mut().animations().remove(old_index);

                            let new_index = self
                                .find_index(new_index)
                                .unwrap_or_else(|| self.animations.len());

                            self.animations().insert(new_index, state);

                            Poll::Ready(Some(VecDiff::Move {
                                old_index,
                                new_index,
                            }))
                        }

                        VecDiff::RemoveAt { index } => {
                            let index = self.find_index(index).expect("Could not find value");

                            if self.should_remove(waker, index) {
                                self.remove_index(index)
                            } else {
                                continue;
                            }
                        }

                        VecDiff::Pop {} => {
                            let index = self.find_last_index().expect("Cannot pop from empty vec");

                            if self.should_remove(waker, index) {
                                self.remove_index(index)
                            } else {
                                continue;
                            }
                        }

                        // TODO maybe it should play remove animation for this ?
                        VecDiff::Clear {} => {
                            self.animations().clear();
                            Poll::Ready(Some(VecDiff::Clear {}))
                        }
                    };
                }
                Poll::Ready(None) => {
                    Pin::set(self.as_mut().signal(), None);
                    break;
                }
                Poll::Pending => {
                    is_done = false;
                    break;
                }
            }
        }

        let mut is_removing = false;

        // TODO make this more efficient (e.g. using a similar strategy as FuturesUnordered)
        // This uses rposition so that way it will return VecDiff::Pop in more situations
        let index = self.as_mut().animations().iter_mut().rposition(|state| {
            if let Some(ref mut future) = state.removing {
                is_removing = true;
                future.poll_unpin(waker).is_ready()
            } else {
                false
            }
        });

        match index {
            Some(index) => self.remove_index(index),
            None => {
                if is_done && !is_removing {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percentage(f64);

impl Percentage {
    #[inline]
    pub fn new(input: f64) -> Self {
        debug_assert!(input >= 0.0 && input <= 1.0);
        Self::new_unchecked(input)
    }

    #[inline]
    pub fn new_unchecked(input: f64) -> Self {
        Percentage(input)
    }

    #[inline]
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(f64) -> f64,
    {
        Self::new(f(self.0))
    }

    #[inline]
    pub fn map_unchecked<F>(self, f: F) -> Self
    where
        F: FnOnce(f64) -> f64,
    {
        Self::new_unchecked(f(self.0))
    }

    #[inline]
    pub fn invert(self) -> Self {
        // TODO use new instead ?
        Self::new_unchecked(1.0 - self.0)
    }

    #[inline]
    pub fn range_inclusive(&self, low: f64, high: f64) -> f64 {
        range_inclusive(self.0, low, high)
    }

    // TODO figure out better name
    #[inline]
    pub fn into_f64(self) -> f64 {
        self.0
    }

    pub fn none_if(self, percentage: f64) -> Option<Self> {
        if self.0 == percentage {
            None
        } else {
            Some(self)
        }
    }
}

#[inline]
fn range_inclusive(percentage: f64, low: f64, high: f64) -> f64 {
    low + (percentage * (high - low))
}

/*pub struct MutableTimestamps<F> {
    callback: Arc<F>,
    animating: Mutex<Option<DiscardOnDrop<CancelableFutureHandle>>>,
}

impl MutableTimestamps<F> where F: FnMut(f64) {
    pub fn new(callback: F) -> Self {
        Self {
            callback: Arc::new(callback),
            animating: Mutex::new(None),
        }
    }

    pub fn stop(&self) {
        let mut lock = self.animating.lock().unwrap();
        *lock = None;
    }

    pub fn start(&self) {
        let mut lock = self.animating.lock().unwrap();

        if let None = animating {
            let callback = self.callback.clone();

            let mut starting_time = None;

            *animating = Some(OnTimestampDiff::new(move |value| callback(value)));
        }
    }
}*/

pub fn timestamps_absolute_difference() -> impl Signal<Item = Option<f64>> {
    let mut starting_time = None;

    timestamps().map(move |current_time| {
        current_time.map(|current_time| {
            let starting_time = *starting_time.get_or_insert(current_time);
            current_time - starting_time
        })
    })
}

pub fn timestamps_difference() -> impl Signal<Item = Option<f64>> {
    let mut previous_time = None;

    timestamps().map(move |current_time| {
        let diff = current_time.map(|current_time| {
            previous_time
                .map(|previous_time| current_time - previous_time)
                .unwrap_or(0.0)
        });

        previous_time = current_time;

        diff
    })
}

pub struct OnTimestampDiff(DiscardOnDrop<CancelableFutureHandle>);

impl OnTimestampDiff {
    pub fn new<F>(mut callback: F) -> Self
    where
        F: FnMut(f64) + 'static,
    {
        OnTimestampDiff(spawn_future(timestamps_absolute_difference().for_each(
            move |diff| {
                if let Some(diff) = diff {
                    callback(diff);
                }

                ready(())
            },
        )))
    }
}

impl fmt::Debug for OnTimestampDiff {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("OnTimestampDiff").finish()
    }
}

#[derive(Debug)]
pub struct MutableAnimationSignal(MutableSignal<Percentage>);

impl Signal for MutableAnimationSignal {
    type Item = Percentage;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, waker: &LocalWaker) -> Poll<Option<Self::Item>> {
        self.0.poll_change_unpin(waker)
    }
}

struct MutableAnimationState {
    playing: bool,
    duration: f64,
    end: Percentage,
    animating: Option<OnTimestampDiff>,
}

struct MutableAnimationInner {
    state: Mutex<MutableAnimationState>,
    value: Mutable<Percentage>,
}

// TODO deref to ReadOnlyMutable ?
// TODO provide read_only() method ?
pub struct MutableAnimation {
    inner: Arc<MutableAnimationInner>,
}

impl fmt::Debug for MutableAnimation {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let state = self.inner.state.lock().unwrap();

        fmt.debug_struct("MutableAnimation")
            .field("playing", &state.playing)
            .field("duration", &state.duration)
            .field("current", &self.inner.value.get())
            .field("end", &state.end)
            .finish()
    }
}

impl MutableAnimation {
    #[inline]
    pub fn new_with_initial(duration: f64, initial: Percentage) -> Self {
        debug_assert!(duration >= 0.0);

        Self {
            inner: Arc::new(MutableAnimationInner {
                state: Mutex::new(MutableAnimationState {
                    playing: true,
                    duration: duration,
                    end: initial,
                    animating: None,
                }),
                value: Mutable::new(initial),
            }),
        }
    }

    #[inline]
    pub fn new(duration: f64) -> Self {
        Self::new_with_initial(duration, Percentage::new_unchecked(0.0))
    }

    #[inline]
    fn raw_clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    #[inline]
    fn stop_animating(lock: &mut MutableAnimationState) {
        lock.animating = None;
    }

    fn start_animating(&self, lock: &mut MutableAnimationState) {
        if lock.playing {
            // TODO use Copy constraint to make value.get() faster ?
            let start: f64 = self.inner.value.get().into_f64();
            let end: f64 = lock.end.into_f64();

            if start != end {
                if lock.duration > 0.0 {
                    let duration = (end - start).abs() * lock.duration;

                    let state = self.raw_clone();

                    lock.animating =
                        Some(OnTimestampDiff::new(move |diff| {
                            let diff = diff / duration;

                            // TODO test the performance of set_neq
                            if diff >= 1.0 {
                                {
                                    let mut lock = state.inner.state.lock().unwrap();
                                    Self::stop_animating(&mut lock);
                                }
                                state.inner.value.set_neq(Percentage::new_unchecked(end));
                            } else {
                                state.inner.value.set_neq(Percentage::new_unchecked(
                                    range_inclusive(diff, start, end),
                                ));
                            }
                        }));
                } else {
                    Self::stop_animating(lock);
                    self.inner.value.set_neq(Percentage::new_unchecked(end));
                }
            } else {
                // TODO is this necessary ?
                Self::stop_animating(lock);
            }
        }
    }

    pub fn set_duration(&self, duration: f64) {
        debug_assert!(duration >= 0.0);

        let mut lock = self.inner.state.lock().unwrap();

        if lock.duration != duration {
            lock.duration = duration;
            self.start_animating(&mut lock);
        }
    }

    #[inline]
    pub fn pause(&self) {
        let mut lock = self.inner.state.lock().unwrap();

        if lock.playing {
            lock.playing = false;
            Self::stop_animating(&mut lock);
        }
    }

    #[inline]
    pub fn play(&self) {
        let mut lock = self.inner.state.lock().unwrap();

        if !lock.playing {
            lock.playing = true;
            self.start_animating(&mut lock);
        }
    }

    fn _jump_to(
        mut lock: &mut MutableAnimationState,
        mutable: &Mutable<Percentage>,
        end: Percentage,
    ) {
        Self::stop_animating(&mut lock);

        lock.end = end;

        mutable.set_neq(end);
    }

    pub fn jump_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap();

        Self::_jump_to(&mut lock, &self.inner.value, end);
    }

    pub fn animate_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap();

        if lock.end != end {
            if lock.duration <= 0.0 {
                Self::_jump_to(&mut lock, &self.inner.value, end);
            } else {
                lock.end = end;
                self.start_animating(&mut lock);
            }
        }
    }

    #[inline]
    pub fn signal(&self) -> MutableAnimationSignal {
        MutableAnimationSignal(self.inner.value.signal())
    }

    #[inline]
    pub fn current_percentage(&self) -> Percentage {
        self.inner.value.get()
    }
}

pub mod easing {
    use super::Percentage;

    // TODO should this use map rather than map_unchecked ?
    #[inline]
    pub fn powi(p: Percentage, n: i32) -> Percentage {
        p.map_unchecked(|p| p.powi(n))
    }

    #[inline]
    pub fn cubic(p: Percentage) -> Percentage {
        powi(p, 3)
    }

    #[inline]
    pub fn out<F>(p: Percentage, f: F) -> Percentage
    where
        F: FnOnce(Percentage) -> Percentage,
    {
        f(p.invert()).invert()
    }

    pub fn in_out<F>(p: Percentage, f: F) -> Percentage
    where
        F: FnOnce(Percentage) -> Percentage,
    {
        p.map_unchecked(|p| {
            if p <= 0.5 {
                f(Percentage::new_unchecked(p * 2.0)).into_f64() / 2.0
            } else {
                1.0 - (f(Percentage::new_unchecked((1.0 - p) * 2.0)).into_f64() / 2.0)
            }
        })
    }
}

// #[derive(Debug)]
// pub struct MutableSpringAnimationSignal(MutableSignal<Percentage>);

// impl Signal for MutableSpringAnimationSignal {
//     type Item = Percentage;

//     #[inline]
//     fn poll_change(mut self: Pin<&mut Self>, waker: &LocalWaker) -> Poll<Option<Self::Item>> {
//         self.0.poll_change_unpin(waker)
//     }
// }

struct MutableSpringAnimationState {
    playing: bool,
    end: Percentage,
    animating: Option<OnTimestampDiff>,
    last_velocity: f64,
}

pub struct SpringConfig {
    stiffness: f64,
    damping: f64,
    mass: f64,
    initial_velocity: f64,
    overshoot_clamping: bool,
    rest_displacement_threshold: f64,
    rest_speed_threshold: f64,
}

struct MutableSpringAnimationInner {
    state: Mutex<MutableSpringAnimationState>,
    value: Mutable<Percentage>,
    config: SpringConfig,
}

pub struct MutableSpringAnimation {
    inner: Arc<MutableSpringAnimationInner>,
}

impl fmt::Debug for MutableSpringAnimation {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let state = self.inner.state.lock().unwrap();

        fmt.debug_struct("MutableAnimation")
            .field("playing", &state.playing)
            .field("current", &self.inner.value.get())
            .field("end", &state.end)
            .field("last_velocity", &state.last_velocity)
            .finish()
    }
}

impl MutableSpringAnimation {
    #[inline]
    pub fn new_with_initial(value: Percentage, config: SpringConfig) -> Self {
        Self {
            inner: Arc::new(MutableSpringAnimationInner {
                state: Mutex::new(MutableSpringAnimationState {
                    playing: true,
                    end: value,
                    animating: None,
                    last_velocity: 0.0,
                }),
                value: Mutable::new(value),
                config,
            }),
        }
    }

    #[inline]
    pub fn new() -> Self {
        let config = SpringConfig {
            stiffness: 100.0,
            damping: 10.0,
            mass: 1.0,
            initial_velocity: 0.0,
            overshoot_clamping: true,
            rest_displacement_threshold: 0.001,
            rest_speed_threshold: 0.001,
        };
        Self::new_with_initial(Percentage::new_unchecked(0.0), config)
    }

    #[inline]
    fn raw_clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    #[inline]
    fn stop_animating(lock: &mut MutableSpringAnimationState) {
        lock.animating = None;
    }

    fn start_animating(&self, lock: &mut MutableSpringAnimationState, reset_velocity: bool) {
        if lock.playing {
            // TODO use Copy constraint to make value.get() faster ?
            let start = self.inner.value.get().into_f64();
            let end = lock.end.into_f64();

            let c = self.inner.config.damping;
            let m = self.inner.config.mass;
            let k = self.inner.config.stiffness;
            let v0 = if reset_velocity {
                -self.inner.config.initial_velocity
            } else {
                -lock.last_velocity
            };

            let mut last_time = 0.0;
            let mut frame_time = 0.0;

            if start != end {
                let state = self.raw_clone();
                lock.animating = Some(OnTimestampDiff::new(move |diff| {
                    // If for some reason we lost a lot of frames (e.g. process large payload or
                    // stopped in the debugger), we only advance by 4 frames worth of
                    // computation and will continue on the next frame. It's better to have it
                    // running at faster speed than jumping to the end.
                    const MAX_STEPS: i64 = 64;
                    let mut now = diff;
                    if now > last_time + MAX_STEPS as f64 {
                        now = last_time + MAX_STEPS as f64;
                    }

                    let delta_time = (now - last_time) / 1000.0;
                    frame_time += delta_time;

                    let zeta = c / (2.0 * (k * m).sqrt()); // damping ratio
                    let omega0 = (k / m).sqrt(); // undamped angular frequency of the oscillator (rad/ms)
                    let omega1 = omega0 * (1.0 - zeta * zeta).sqrt(); // exponential decay
                    let x0 = end - start; // calculate the oscillation from x0 = 1 to x = 0
                    let t = frame_time;
                    let value: f64;
                    let velocity: f64;

                    if zeta < 1.0 {
                        // Under damped
                        let envelope = (-zeta * omega0 * t).exp();
                        value = end
                            - envelope
                                * (((v0 + zeta * omega0 * x0) / omega1) * (omega1 * t).sin()
                                    + x0 * (omega1 * t).cos());
                        // This looks crazy -- it's actually just the derivative of the
                        // oscillation function
                        velocity = zeta
                            * omega0
                            * envelope
                            * (((omega1 * t).sin() * (v0 + zeta * omega0 * x0)) / omega1
                                + x0 * (omega1 * t).cos())
                            - envelope
                                * ((omega1 * t).cos() * (v0 + zeta * omega0 * x0)
                                    - omega1 * x0 * (omega1 * t).sin());
                    } else {
                        // Critically damped
                        let envelope = (-omega0 * t).exp();
                        value = end - envelope * (x0 + (v0 + omega0 * x0) * t);
                        velocity =
                            envelope * (v0 * (t * omega0 - 1.0) + t * x0 * (omega0 * omega0));
                    }

                    last_time = now;
                    let mut lock = state.inner.state.lock().unwrap();
                    lock.last_velocity = velocity;

                    state.inner.value.set_neq(Percentage::new_unchecked(value));

                    // Conditions for stopping the spring animation
                    let mut is_overshooting = false;
                    if state.inner.config.overshoot_clamping && state.inner.config.stiffness != 0.0
                    {
                        if start < end {
                            is_overshooting = value > end;
                        } else {
                            is_overshooting = value < end;
                        }
                    }
                    let is_velocity = velocity.abs() <= state.inner.config.rest_speed_threshold;
                    let mut is_displacement = true;
                    if state.inner.config.stiffness != 0.0 {
                        is_displacement =
                            (end - value).abs() <= state.inner.config.rest_displacement_threshold;
                    }

                    if is_overshooting || (is_velocity && is_displacement) {
                        if state.inner.config.stiffness != 0.0 {
                            // Ensure that we end up with a round value
                            lock.last_velocity = 0.0;
                            state.inner.value.set_neq(Percentage::new_unchecked(end));
                        }
                        Self::stop_animating(&mut lock);
                    }
                }));
            } else {
                // TODO is this necessary ?
                Self::stop_animating(lock);
            }
        }
    }

    #[inline]
    pub fn pause(&self) {
        let mut lock = self.inner.state.lock().unwrap();

        if lock.playing {
            lock.playing = false;
            Self::stop_animating(&mut lock);
        }
    }

    #[inline]
    pub fn play(&self) {
        let mut lock = self.inner.state.lock().unwrap();

        if !lock.playing {
            lock.playing = true;
            self.start_animating(&mut lock, false);
        }
    }

    fn _jump_to(
        mut lock: &mut MutableSpringAnimationState,
        mutable: &Mutable<Percentage>,
        end: Percentage,
    ) {
        Self::stop_animating(&mut lock);

        lock.end = end;

        mutable.set_neq(end);
    }

    pub fn jump_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap();

        Self::_jump_to(&mut lock, &self.inner.value, end);
    }

    pub fn animate_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap();

        if lock.end != end {
            lock.end = end;
            self.start_animating(&mut lock, true);
        }
    }

    pub fn animate_to_contineous(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap();

        if lock.end != end {
            lock.end = end;
            self.start_animating(&mut lock, false);
        }
    }

    #[inline]
    pub fn signal(&self) -> MutableAnimationSignal {
        MutableAnimationSignal(self.inner.value.signal())
    }

    #[inline]
    pub fn current_percentage(&self) -> Percentage {
        self.inner.value.get()
    }
}

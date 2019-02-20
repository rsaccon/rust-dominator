#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate dominator;
#[macro_use]
extern crate futures_signals;

use std::rc::Rc;
use futures::future::ready;
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::MutableVec;
use dominator::traits::*;
use dominator::Dom;
use dominator::events::{MouseOverEvent, MouseOutEvent};
use dominator::animation::{Percentage, easing};
use dominator::animation::{MutableAnimation, MutableSpringAnimation, AnimatedMapBroadcaster};

struct State {
}

impl State {
    fn new() -> Self {
        State {}
    }

    fn render(_state: Self) -> Dom {
        let hover_animation_1 = Rc::new(MutableAnimation::new(500.0));
        let hover_animation_spring_1 = Rc::new(MutableSpringAnimation::new());

        html!("div", {
            .children(&mut [
                html!("div", {
                    .event(clone!(hover_animation_1 => move |_: MouseOverEvent| {
                        hover_animation_1.animate_to(Percentage::new(1.0));
                    }))
                    .event(clone!(hover_animation_1 => move |_: MouseOutEvent| {
                        hover_animation_1.animate_to(Percentage::new(0.0));
                    }))
                    .style("background-color", "yellow")
                    .style("margin", "10px 0")
                    .style_signal("width", hover_animation_1.signal()
                        .map(|t| easing::in_out(t, easing::cubic))
                        .map(|t| Some(format!("{}%", t.range_inclusive(0.0, 100.0)))))
                    .text("EasingInOut")
                }),
                html!("div", {
                    .event(clone!(hover_animation_spring_1 => move |_: MouseOverEvent| {
                        hover_animation_spring_1.animate_to(Percentage::new(1.0));
                    }))
                    .event(clone!(hover_animation_spring_1 => move |_: MouseOutEvent| {
                        hover_animation_spring_1.animate_to(Percentage::new(0.0));
                    }))
                    .style("background-color", "orange")
                    .style("margin", "5px 0")
                    .style_signal("width", hover_animation_spring_1.signal()
                        .map(|t| Some(format!("{}%", t.range_inclusive(0.0, 100.0)))))
                    .text("spring500Pixels")
                    }),
            ])
        })
    }
}

fn main() {
    let state = State::new();

    dominator::append_dom(&dominator::body(), State::render(state));
}
use callbacks::Callbacks;
use discard::{Discard, DiscardOnDrop};
use dom_operations;
use futures_channel::oneshot;
use futures_core::future::Future;
use futures_core::task::LocalWaker;
use futures_core::Poll;
use futures_signals::signal::{not, Signal};
use futures_signals::signal_vec::SignalVec;
use futures_util::FutureExt;
use operations;
use operations::for_each;
use operations::{spawn_future, FnDiscard, ValueDiscard};
use std::convert::AsRef;
use std::marker::PhantomData;
use std::pin::Pin;
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::web::event::ConcreteEvent;
use stdweb::web::{
    window, Element, EventTarget, HtmlElement, IElement, IEventTarget, IHtmlElement, INode, Node,
    TextNode,
};
use stdweb::{JsSerialize, Once, Reference, Value};
use traits::*;

pub struct RefFn<A, B, C>
where
    B: ?Sized,
{
    value: A,
    callback: C,
    return_value: PhantomData<B>,
}

impl<A, B, C> RefFn<A, B, C>
where
    B: ?Sized,
    C: Fn(&A) -> &B,
{
    #[inline]
    pub fn new(value: A, callback: C) -> Self {
        Self {
            value,
            callback,
            return_value: PhantomData,
        }
    }

    #[inline]
    pub fn call_ref(&self) -> &B {
        (self.callback)(&self.value)
    }

    /*pub fn map<D, E>(self, callback: E) -> RefFn<A, impl Fn(&A) -> &D>
        where D: ?Sized,
              E: Fn(&B) -> &D {

        let old_callback = self.callback;

        RefFn {
            value: self.value,
            callback: move |value| callback(old_callback(value)),
        }
    }*/
}

/*impl<A, B, C> Deref for RefFn<A, C> where B: ?Sized, C: Fn(&A) -> &B {
    type Target = B;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.call_ref()
    }
}*/

/*impl<A, B, C> AsRef<B> for RefFn<A, C> where B: ?Sized, C: Fn(&A) -> &B {
    #[inline]
    fn as_ref(&self) -> &B {
        self.call_ref()
    }
}*/

// TODO this should be in stdweb
#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "CSSStyleRule")]
pub struct CssStyleRule(Reference);

/// A reference to an SVG Element.
///
/// [(JavaScript docs)](https://developer.mozilla.org/en-US/docs/Web/API/SVGElement)
// TODO move this into stdweb
#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "SVGElement")]
#[reference(subclass_of(EventTarget, Node, Element))]
pub struct SvgElement(Reference);

impl IEventTarget for SvgElement {}
impl INode for SvgElement {}
impl IElement for SvgElement {}

// TODO move this into stdweb
#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "URL")]
pub struct Url(Reference);

// TODO create_object_url, revoke_object_url, and search_params
impl Url {
    #[inline]
    pub fn new(url: &str) -> Self {
        js!( return new URL( @{url} ); ).try_into().unwrap()
    }

    #[inline]
    pub fn hash(&self) -> String {
        js!( return @{&self.0}.hash; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_hash(&self, hash: &str) {
        js! { @(no_return)
            @{&self.0}.hash = @{hash};
        }
    }

    #[inline]
    pub fn host(&self) -> String {
        js!( return @{&self.0}.host; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_host(&self, host: &str) {
        js! { @(no_return)
            @{&self.0}.host = @{host};
        }
    }

    #[inline]
    pub fn hostname(&self) -> String {
        js!( return @{&self.0}.hostname; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_hostname(&self, hostname: &str) {
        js! { @(no_return)
            @{&self.0}.hostname = @{hostname};
        }
    }

    #[inline]
    pub fn href(&self) -> String {
        js!( return @{&self.0}.href; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_href(&self, href: &str) {
        js! { @(no_return)
            @{&self.0}.href = @{href};
        }
    }

    #[inline]
    pub fn origin(&self) -> String {
        js!( return @{&self.0}.origin; ).try_into().unwrap()
    }

    #[inline]
    pub fn password(&self) -> String {
        js!( return @{&self.0}.password; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_password(&self, password: &str) {
        js! { @(no_return)
            @{&self.0}.password = @{password};
        }
    }

    #[inline]
    pub fn pathname(&self) -> String {
        js!( return @{&self.0}.pathname; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_pathname(&self, pathname: &str) {
        js! { @(no_return)
            @{&self.0}.pathname = @{pathname};
        }
    }

    #[inline]
    pub fn port(&self) -> String {
        js!( return @{&self.0}.port; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_port(&self, port: &str) {
        js! { @(no_return)
            @{&self.0}.port = @{port};
        }
    }

    #[inline]
    pub fn protocol(&self) -> String {
        js!( return @{&self.0}.protocol; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_protocol(&self, protocol: &str) {
        js! { @(no_return)
            @{&self.0}.protocol = @{protocol};
        }
    }

    #[inline]
    pub fn search(&self) -> String {
        js!( return @{&self.0}.search; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_search(&self, search: &str) {
        js! { @(no_return)
            @{&self.0}.search = @{search};
        }
    }

    #[inline]
    pub fn username(&self) -> String {
        js!( return @{&self.0}.username; ).try_into().unwrap()
    }

    #[inline]
    pub fn set_username(&self, username: &str) {
        js! { @(no_return)
            @{&self.0}.username = @{username};
        }
    }
}

impl ::std::fmt::Display for Url {
    #[inline]
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        self.href().fmt(f)
    }
}

// https://developer.mozilla.org/en-US/docs/Web/API/Document/createElementNS#Valid%20Namespace%20URIs
pub const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

// 32-bit signed int
pub const HIGHEST_ZINDEX: &str = "2147483647";

lazy_static! {
    static ref HIDDEN_CLASS: String = class! {
        .style_important("display", "none")
    };
}

// TODO this should be in stdweb
// TODO this should return HtmlBodyElement
pub fn body() -> HtmlElement {
    js! ( return document.body; ).try_into().unwrap()
}

pub struct DomHandle {
    parent: Node,
    dom: Dom,
}

impl Discard for DomHandle {
    #[inline]
    fn discard(self) {
        self.parent.remove_child(&self.dom.element).unwrap();
        self.dom.callbacks.discard();
    }
}

#[inline]
pub fn append_dom<A: INode>(parent: &A, mut dom: Dom) -> DomHandle {
    parent.append_child(&dom.element);

    dom.callbacks.trigger_after_insert();

    // This prevents it from triggering after_remove
    dom.callbacks.leak();

    DomHandle {
        parent: parent.as_node().clone(),
        dom,
    }
}

struct IsWindowLoadedEvent {
    callback: Value,
}

impl IsWindowLoadedEvent {
    #[inline]
    fn new<F>(callback: F) -> Self
    where
        F: FnOnce() + 'static,
    {
        // TODO use a proper type for the event
        let callback = move |_: Value| {
            callback();
        };

        Self {
            callback: js!(
                var callback = @{Once(callback)};
                addEventListener("load", callback, true);
                return callback;
            ),
        }
    }
}

impl Drop for IsWindowLoadedEvent {
    fn drop(&mut self) {
        js! { @(no_return)
            var callback = @{&self.callback};
            removeEventListener("load", callback, true);
            callback.drop();
        }
    }
}

enum IsWindowLoaded {
    Initial {},
    Pending {
        receiver: oneshot::Receiver<()>,
        _event: IsWindowLoadedEvent,
    },
    Done {},
}

impl Signal for IsWindowLoaded {
    type Item = bool;

    fn poll_change(self: Pin<&mut Self>, waker: &LocalWaker) -> Poll<Option<Self::Item>> {
        // Safe to call `get_mut_unchecked` because we won't move the futures.
        // TODO verify the safety of this
        let this = unsafe { Pin::get_unchecked_mut(self) };

        let result = match this {
            IsWindowLoaded::Initial {} => {
                let is_ready: bool = js!( return document.readyState === "complete"; )
                    .try_into()
                    .unwrap();

                if is_ready {
                    Poll::Ready(Some(true))
                } else {
                    let (sender, receiver) = oneshot::channel();

                    *this = IsWindowLoaded::Pending {
                        receiver,
                        _event: IsWindowLoadedEvent::new(move || {
                            // TODO test this
                            sender.send(()).unwrap();
                        }),
                    };

                    Poll::Ready(Some(false))
                }
            }
            IsWindowLoaded::Pending { receiver, .. } => {
                receiver.poll_unpin(waker).map(|_| Some(true))
            }
            IsWindowLoaded::Done {} => Poll::Ready(None),
        };

        if let Poll::Ready(Some(true)) = result {
            *this = IsWindowLoaded::Done {};
        }

        result
    }
}

#[inline]
pub fn is_window_loaded() -> impl Signal<Item = bool> {
    IsWindowLoaded::Initial {}
}

#[inline]
pub fn text(value: &str) -> Dom {
    Dom::new(
        js!( return document.createTextNode(@{value}); )
            .try_into()
            .unwrap(),
    )
}

// TODO should this inline ?
pub fn text_signal<A, B>(value: B) -> Dom
where
    A: AsStr,
    B: Signal<Item = A> + 'static,
{
    let element: TextNode = js!( return document.createTextNode(""); )
        .try_into()
        .unwrap();

    let mut callbacks = Callbacks::new();

    {
        let element = element.clone();

        callbacks.after_remove(for_each(value, move |value| {
            let value = value.as_str();
            dom_operations::set_text(&element, value);
        }));
    }

    Dom {
        element: element.into(),
        callbacks: callbacks,
    }
}

#[derive(Debug)]
pub struct Dom {
    pub(crate) element: Node,
    pub(crate) callbacks: Callbacks,
}

impl Dom {
    #[inline]
    pub fn new(element: Node) -> Self {
        Self {
            element,
            callbacks: Callbacks::new(),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        // TODO is there a better way of doing this ?
        Self::new(
            js!( return document.createComment(""); )
                .try_into()
                .unwrap(),
        )
    }

    #[inline]
    pub fn with_state<A, F>(mut state: A, initializer: F) -> Dom
    where
        A: 'static,
        F: FnOnce(&mut A) -> Dom,
    {
        let mut dom = initializer(&mut state);

        dom.callbacks.after_remove(ValueDiscard::new(state));

        dom
    }
}

struct EventListenerHandle<A>
where
    A: AsRef<Reference>,
{
    event: &'static str,
    element: A,
    listener: Value,
}

impl<A> Discard for EventListenerHandle<A>
where
    A: AsRef<Reference>,
{
    #[inline]
    fn discard(self) {
        js! { @(no_return)
            var listener = @{&self.listener};
            @{self.element.as_ref()}.removeEventListener(@{self.event}, listener);
            listener.drop();
        }
    }
}

// TODO create HTML / SVG specific versions of this ?
#[inline]
pub fn create_element_ns<A: IElement>(name: &str, namespace: &str) -> A
where
    <A as TryFrom<Value>>::Error: ::std::fmt::Debug,
{
    dom_operations::create_element_ns(name, namespace)
}

fn set_option<A, B, C, D, F>(element: &A, callbacks: &mut Callbacks, value: D, mut f: F)
where
    A: Clone + 'static,
    C: OptionStr<Output = B>,
    D: Signal<Item = C> + 'static,
    F: FnMut(&A, Option<B>) + 'static,
{
    let element = element.clone();

    let mut is_set = false;

    callbacks.after_remove(for_each(value, move |value| {
        let value = value.into_option();

        if value.is_some() {
            is_set = true;
        } else if is_set {
            is_set = false;
        } else {
            return;
        }

        f(&element, value);
    }));
}

fn set_style<A, B, C>(element: &A, name: &B, value: C, important: bool)
where
    A: AsRef<Reference>,
    B: MultiStr,
    C: MultiStr,
{
    let mut names = vec![];
    let mut values = vec![];

    let okay = name.any(|name| {
        value.any(|value| {
            if dom_operations::try_set_style(element, name, value, important) {
                true
            } else {
                names.push(name.to_string());
                values.push(value.to_string());
                false
            }
        })
    });

    if !okay {
        // TODO maybe make this configurable
        panic!(
            "style is incorrect:\n  names: {}\n  values: {}",
            names.join(", "),
            values.join(", ")
        );
    }
}

fn set_style_signal<A, B, C, D, E>(
    element: &A,
    callbacks: &mut Callbacks,
    name: B,
    value: E,
    important: bool,
) where
    A: AsRef<Reference> + Clone + 'static,
    B: MultiStr + 'static,
    C: MultiStr,
    D: OptionStr<Output = C>,
    E: Signal<Item = D> + 'static,
{
    set_option(
        element,
        callbacks,
        value,
        move |element, value| match value {
            Some(value) => {
                set_style(element, &name, value, important);
            }
            None => {
                name.each(|name| {
                    dom_operations::remove_style(element, name);
                });
            }
        },
    );
}

pub struct DomBuilder<A> {
    element: A,
    callbacks: Callbacks,
    // TODO verify this with static types instead ?
    has_children: bool,
}

impl<A> DomBuilder<A> {
    #[inline]
    pub fn new(value: A) -> Self {
        Self {
            element: value,
            callbacks: Callbacks::new(),
            has_children: false,
        }
    }

    // TODO maybe inline this ?
    // TODO replace with element.add_event_listener
    fn _event<B, T, F>(&mut self, element: B, listener: F)
    where
        B: IEventTarget + 'static,
        T: ConcreteEvent,
        F: FnMut(T) + 'static,
    {
        let listener = js!(
            var listener = @{listener};
            @{element.as_ref()}.addEventListener(@{T::EVENT_TYPE}, listener);
            return listener;
        );

        self.callbacks.after_remove(EventListenerHandle {
            event: T::EVENT_TYPE,
            element,
            listener,
        });
    }

    // TODO add this to the StylesheetBuilder and ClassBuilder too
    #[inline]
    pub fn global_event<T, F>(mut self, listener: F) -> Self
    where
        T: ConcreteEvent,
        F: FnMut(T) + 'static,
    {
        self._event(window(), listener);
        self
    }

    #[inline]
    pub fn future<F>(mut self, future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        self.callbacks
            .after_remove(DiscardOnDrop::leak(spawn_future(future)));
        self
    }

    #[inline]
    pub fn apply<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        self.apply_if(true, f)
    }

    #[inline]
    pub fn apply_if<F>(self, test: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if test {
            f(self)
        } else {
            self
        }
    }

    #[deprecated(since = "0.3.2", note = "Use apply instead")]
    #[allow(deprecated)]
    #[inline]
    pub fn mixin<B: Mixin<Self>>(self, mixin: B) -> Self {
        self.mixin_if(true, mixin)
    }

    #[deprecated(since = "0.3.2", note = "Use apply_if instead")]
    #[allow(deprecated)]
    #[inline]
    pub fn mixin_if<B: Mixin<Self>>(self, test: bool, mixin: B) -> Self {
        if test {
            mixin.apply(self)
        } else {
            self
        }
    }
}

impl<A: Clone> DomBuilder<A> {
    #[inline]
    pub fn with_element<B, F>(self, f: F) -> B
    where
        F: FnOnce(Self, A) -> B,
    {
        let element = self.element.clone();
        f(self, element)
    }

    #[inline]
    pub fn before_inserted<F>(self, f: F) -> Self
    where
        F: FnOnce(A),
    {
        let element = self.element.clone();
        f(element);
        self
    }
}

impl<A: Clone + 'static> DomBuilder<A> {
    #[inline]
    pub fn after_inserted<F>(mut self, f: F) -> Self
    where
        F: FnOnce(A) + 'static,
    {
        let element = self.element.clone();
        self.callbacks.after_insert(move |_| f(element));
        self
    }

    #[inline]
    pub fn after_removed<F>(mut self, f: F) -> Self
    where
        F: FnOnce(A) + 'static,
    {
        let element = self.element.clone();
        self.callbacks
            .after_remove(FnDiscard::new(move || f(element)));
        self
    }
}

impl<A: Into<Node>> DomBuilder<A> {
    #[inline]
    pub fn into_dom(self) -> Dom {
        Dom {
            element: self.element.into(),
            callbacks: self.callbacks,
        }
    }
}

// TODO make this JsSerialize rather than AsRef<Reference> ?
impl<A: AsRef<Reference>> DomBuilder<A> {
    #[inline]
    pub fn property<B, C>(self, name: B, value: C) -> Self
    where
        B: MultiStr,
        C: JsSerialize,
    {
        name.each(|name| {
            dom_operations::set_property(&self.element, name, &value);
        });
        self
    }
}

impl<A: AsRef<Reference> + Clone + 'static> DomBuilder<A> {
    fn set_property_signal<B, C, D>(&mut self, name: B, value: D)
    where
        B: MultiStr + 'static,
        C: JsSerialize,
        D: Signal<Item = C> + 'static,
    {
        let element = self.element.clone();

        self.callbacks.after_remove(for_each(value, move |value| {
            name.each(|name| {
                dom_operations::set_property(&element, name, &value);
            });
        }));
    }

    #[inline]
    pub fn property_signal<B, C, D>(mut self, name: B, value: D) -> Self
    where
        B: MultiStr + 'static,
        C: JsSerialize,
        D: Signal<Item = C> + 'static,
    {
        self.set_property_signal(name, value);
        self
    }
}

impl<A: IEventTarget + Clone + 'static> DomBuilder<A> {
    #[inline]
    pub fn event<T, F>(mut self, listener: F) -> Self
    where
        T: ConcreteEvent,
        F: FnMut(T) + 'static,
    {
        // TODO is this clone correct ?
        let element = self.element.clone();
        self._event(element, listener);
        self
    }
}

impl<A: INode> DomBuilder<A> {
    // TODO figure out how to make this owned rather than &mut
    #[inline]
    pub fn children<'a, B: IntoIterator<Item = &'a mut Dom>>(mut self, children: B) -> Self {
        assert_eq!(self.has_children, false);
        self.has_children = true;

        operations::insert_children_iter(&self.element, &mut self.callbacks, children);
        self
    }

    #[inline]
    pub fn text(self, value: &str) -> Self {
        // TODO make this more efficient ?
        self.children(&mut [text(value)])
    }

    #[inline]
    pub fn text_signal<B, C>(self, value: C) -> Self
    where
        B: AsStr,
        C: Signal<Item = B> + 'static,
    {
        // TODO make this more efficient ?
        self.children(&mut [text_signal(value)])
    }
}

impl<A: INode + Clone + 'static> DomBuilder<A> {
    #[inline]
    pub fn children_signal_vec<B>(mut self, children: B) -> Self
    where
        B: SignalVec<Item = Dom> + 'static,
    {
        assert_eq!(self.has_children, false);
        self.has_children = true;

        operations::insert_children_signal_vec(&self.element, &mut self.callbacks, children);
        self
    }
}

impl<A: IElement> DomBuilder<A> {
    #[inline]
    pub fn attribute<B>(self, name: B, value: &str) -> Self
    where
        B: MultiStr,
    {
        name.each(|name| {
            dom_operations::set_attribute(&self.element, name, value);
        });
        self
    }

    #[inline]
    pub fn attribute_namespace<B>(self, namespace: &str, name: B, value: &str) -> Self
    where
        B: MultiStr,
    {
        name.each(|name| {
            dom_operations::set_attribute_ns(&self.element, namespace, name, value);
        });
        self
    }

    #[inline]
    pub fn class<B>(self, name: B) -> Self
    where
        B: MultiStr,
    {
        name.each(|name| {
            dom_operations::add_class(&self.element, name);
        });
        self
    }
}

impl<A: IElement + Clone + 'static> DomBuilder<A> {
    fn set_attribute_signal<B, C, D, E>(&mut self, name: B, value: E)
    where
        B: MultiStr + 'static,
        C: AsStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        set_option(
            &self.element,
            &mut self.callbacks,
            value,
            move |element, value| match value {
                Some(value) => {
                    let value = value.as_str();

                    name.each(|name| {
                        dom_operations::set_attribute(element, &name, value);
                    });
                }
                None => {
                    name.each(|name| dom_operations::remove_attribute(element, &name));
                }
            },
        );
    }

    #[inline]
    pub fn attribute_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
    where
        B: MultiStr + 'static,
        C: AsStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        self.set_attribute_signal(name, value);
        self
    }

    fn set_attribute_namespace_signal<B, C, D, E>(&mut self, namespace: &str, name: B, value: E)
    where
        B: MultiStr + 'static,
        C: AsStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        let namespace = namespace.to_owned();

        set_option(
            &self.element,
            &mut self.callbacks,
            value,
            move |element, value| match value {
                Some(value) => {
                    let value = value.as_str();

                    name.each(|name| {
                        dom_operations::set_attribute_ns(element, &namespace, &name, value);
                    });
                }
                None => {
                    name.each(|name| {
                        dom_operations::remove_attribute_ns(element, &namespace, &name);
                    });
                }
            },
        );
    }

    #[inline]
    pub fn attribute_namespace_signal<B, C, D, E>(
        mut self,
        namespace: &str,
        name: B,
        value: E,
    ) -> Self
    where
        B: MultiStr + 'static,
        C: AsStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        self.set_attribute_namespace_signal(namespace, name, value);
        self
    }

    fn set_class_signal<B, C>(&mut self, name: B, value: C)
    where
        B: MultiStr + 'static,
        C: Signal<Item = bool> + 'static,
    {
        let element = self.element.clone();

        let mut is_set = false;

        self.callbacks.after_remove(for_each(value, move |value| {
            if value {
                if !is_set {
                    is_set = true;

                    name.each(|name| {
                        dom_operations::add_class(&element, name);
                    });
                }
            } else {
                if is_set {
                    is_set = false;

                    name.each(|name| {
                        dom_operations::remove_class(&element, name);
                    });
                }
            }
        }));
    }

    #[inline]
    pub fn class_signal<B, C>(mut self, name: B, value: C) -> Self
    where
        B: MultiStr + 'static,
        C: Signal<Item = bool> + 'static,
    {
        self.set_class_signal(name, value);
        self
    }

    // TODO use OptionStr ?
    fn set_scroll_signal<B, F>(&mut self, signal: B, mut f: F)
    where
        B: Signal<Item = Option<f64>> + 'static,
        F: FnMut(&A, f64) + 'static,
    {
        let element = self.element.clone();

        // This needs to use `after_insert` because scrolling an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |callbacks| {
            callbacks.after_remove(for_each(signal, move |value| {
                if let Some(value) = value {
                    f(&element, value);
                }
            }));
        });
    }

    #[inline]
    pub fn scroll_left_signal<B>(mut self, signal: B) -> Self
    where
        B: Signal<Item = Option<f64>> + 'static,
    {
        self.set_scroll_signal(signal, IElement::set_scroll_left);
        self
    }

    #[inline]
    pub fn scroll_top_signal<B>(mut self, signal: B) -> Self
    where
        B: Signal<Item = Option<f64>> + 'static,
    {
        self.set_scroll_signal(signal, IElement::set_scroll_top);
        self
    }
}

impl<A: IHtmlElement> DomBuilder<A> {
    #[inline]
    pub fn style<B, C>(self, name: B, value: C) -> Self
    where
        B: MultiStr,
        C: MultiStr,
    {
        set_style(&self.element, &name, value, false);
        self
    }

    #[inline]
    pub fn style_important<B, C>(self, name: B, value: C) -> Self
    where
        B: MultiStr,
        C: MultiStr,
    {
        set_style(&self.element, &name, value, true);
        self
    }

    // TODO make this more efficient ?
    #[inline]
    pub fn visible(self, value: bool) -> Self {
        if value {
            // TODO remove the class somehow ?
            self
        } else {
            self.class(&*HIDDEN_CLASS)
        }
    }
}

impl<A: IHtmlElement + Clone + 'static> DomBuilder<A> {
    #[inline]
    pub fn style_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
    where
        B: MultiStr + 'static,
        C: MultiStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        set_style_signal(&self.element, &mut self.callbacks, name, value, false);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
    where
        B: MultiStr + 'static,
        C: MultiStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        set_style_signal(&self.element, &mut self.callbacks, name, value, true);
        self
    }

    // TODO remove the `value` argument ?
    #[inline]
    pub fn focused(mut self, value: bool) -> Self {
        let element = self.element.clone();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |_| {
            // TODO avoid updating if the focused state hasn't changed ?
            dom_operations::set_focused(&element, value);
        });

        self
    }

    fn set_focused_signal<B>(&mut self, value: B)
    where
        B: Signal<Item = bool> + 'static,
    {
        let element = self.element.clone();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |callbacks| {
            // TODO verify that this is correct under all circumstances
            callbacks.after_remove(for_each(value, move |value| {
                // TODO avoid updating if the focused state hasn't changed ?
                dom_operations::set_focused(&element, value);
            }));
        });
    }

    #[inline]
    pub fn focused_signal<B>(mut self, value: B) -> Self
    where
        B: Signal<Item = bool> + 'static,
    {
        self.set_focused_signal(value);
        self
    }

    // TODO make this more efficient ?
    #[inline]
    pub fn visible_signal<B>(self, value: B) -> Self
    where
        B: Signal<Item = bool> + 'static,
    {
        self.class_signal(&*HIDDEN_CLASS, not(value))
    }
}

// TODO better warning message for must_use
#[must_use]
pub struct StylesheetBuilder {
    element: CssStyleRule,
    callbacks: Callbacks,
}

// TODO remove the CssStyleRule when this is discarded
impl StylesheetBuilder {
    #[inline]
    pub fn new(selector: &str) -> Self {
        lazy_static! {
            // TODO better static type for this
            static ref STYLESHEET: Reference = js!(
                // TODO use createElementNS ?
                var e = document.createElement("style");
                e.type = "text/css";
                document.head.appendChild(e);
                return e.sheet;
            ).try_into().unwrap();
        }

        Self {
            element: js!(
                var stylesheet = @{&*STYLESHEET};
                var length = stylesheet.cssRules.length;
                stylesheet.insertRule(@{selector} + "{}", length);
                return stylesheet.cssRules[length];
            )
            .try_into()
            .unwrap(),
            callbacks: Callbacks::new(),
        }
    }

    #[inline]
    pub fn style<B, C>(self, name: B, value: C) -> Self
    where
        B: MultiStr,
        C: MultiStr,
    {
        set_style(&self.element, &name, value, false);
        self
    }

    #[inline]
    pub fn style_important<B, C>(self, name: B, value: C) -> Self
    where
        B: MultiStr,
        C: MultiStr,
    {
        set_style(&self.element, &name, value, true);
        self
    }

    #[inline]
    pub fn style_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
    where
        B: MultiStr + 'static,
        C: MultiStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        set_style_signal(&self.element, &mut self.callbacks, name, value, false);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
    where
        B: MultiStr + 'static,
        C: MultiStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        set_style_signal(&self.element, &mut self.callbacks, name, value, true);
        self
    }

    // TODO return a Handle
    #[inline]
    pub fn done(mut self) {
        self.callbacks.trigger_after_insert();

        // This prevents it from triggering after_remove
        self.callbacks.leak();
    }
}

// TODO better warning message for must_use
#[must_use]
pub struct ClassBuilder {
    stylesheet: StylesheetBuilder,
    class_name: String,
}

impl ClassBuilder {
    #[inline]
    pub fn new(pseudo_class: Option<&str>) -> Self {
        let class_name = {
            use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

            // TODO replace this with a global counter in JavaScript ?
            lazy_static! {
                // TODO can this be made more efficient ?
                // TODO use AtomicU32 instead ?
                static ref CLASS_ID: AtomicUsize = ATOMIC_USIZE_INIT;
            }

            // TODO check for overflow ?
            let id = CLASS_ID.fetch_add(1, Ordering::Relaxed);

            // TODO make this more efficient ?
            format!("__class_{}__", id)
        };

        let selector = match pseudo_class {
            None => class_name.clone(),
            Some(value) => format!("{}:{}", class_name, value),
        };

        Self {
            // TODO make this more efficient ?
            stylesheet: StylesheetBuilder::new(&format!(".{}", selector)),
            class_name,
        }
    }

    #[inline]
    pub fn style<B, C>(mut self, name: B, value: C) -> Self
    where
        B: MultiStr,
        C: MultiStr,
    {
        self.stylesheet = self.stylesheet.style(name, value);
        self
    }

    #[inline]
    pub fn style_important<B, C>(mut self, name: B, value: C) -> Self
    where
        B: MultiStr,
        C: MultiStr,
    {
        self.stylesheet = self.stylesheet.style_important(name, value);
        self
    }

    #[inline]
    pub fn style_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
    where
        B: MultiStr + 'static,
        C: MultiStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        self.stylesheet = self.stylesheet.style_signal(name, value);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
    where
        B: MultiStr + 'static,
        C: MultiStr,
        D: OptionStr<Output = C>,
        E: Signal<Item = D> + 'static,
    {
        self.stylesheet = self.stylesheet.style_important_signal(name, value);
        self
    }

    // TODO return a Handle ?
    #[inline]
    pub fn done(self) -> String {
        self.stylesheet.done();
        self.class_name
    }
}

#[cfg(test)]
mod tests {
    use super::{create_element_ns, text_signal, DomBuilder, RefFn, HTML_NAMESPACE};
    use futures_signals::signal::{always, SignalExt};
    use stdweb::web::{HtmlElement, IHtmlElement};

    #[test]
    fn mixin() {
        let a: DomBuilder<HtmlElement> = DomBuilder::new(create_element_ns("div", HTML_NAMESPACE));

        fn my_mixin<A: IHtmlElement>(builder: DomBuilder<A>) -> DomBuilder<A> {
            builder.style("foo", "bar")
        }

        a.mixin(my_mixin);
    }

    #[test]
    fn text_signal_types() {
        text_signal(always("foo"));
        text_signal(always("foo".to_owned()));
        text_signal(always("foo".to_owned()).map(|x| RefFn::new(x, |x| x.as_str())));
        //text_signal(always(Arc::new("foo")));
        //text_signal(always(Arc::new("foo".to_owned())));
        //text_signal(always(Rc::new("foo")));
        //text_signal(always(Rc::new("foo".to_owned())));
        //text_signal(always(Box::new("foo")));
        //text_signal(always(Box::new("foo".to_owned())));
        //text_signal(always(Cow::Borrowed(&"foo")));
        //text_signal(always(Cow::Owned::<String>("foo".to_owned())));
    }

    #[test]
    fn property_signal_types() {
        let _a: DomBuilder<HtmlElement> = DomBuilder::new(create_element_ns("div", HTML_NAMESPACE))
            .property("foo", "hi")
            .property("foo", 5)
            .property(["foo", "-webkit-foo", "-ms-foo"], "hi")
            .property_signal("foo", always("hi"))
            .property_signal("foo", always(5))
            .property_signal("foo", always(Some("hi")))
            .property_signal(["foo", "-webkit-foo", "-ms-foo"], always("hi"))
            .property_signal(["foo", "-webkit-foo", "-ms-foo"], always(5))
            .property_signal(["foo", "-webkit-foo", "-ms-foo"], always(Some("hi")));
    }

    #[test]
    fn attribute_signal_types() {
        let _a: DomBuilder<HtmlElement> = DomBuilder::new(create_element_ns("div", HTML_NAMESPACE))
            .attribute("foo", "hi")
            .attribute(["foo", "-webkit-foo", "-ms-foo"], "hi")
            .attribute_signal("foo", always("hi"))
            .attribute_signal("foo", always(Some("hi")))
            .attribute_signal(["foo", "-webkit-foo", "-ms-foo"], always("hi"))
            .attribute_signal(["foo", "-webkit-foo", "-ms-foo"], always(Some("hi")));
    }

    #[test]
    fn class_signal_types() {
        let _a: DomBuilder<HtmlElement> = DomBuilder::new(create_element_ns("div", HTML_NAMESPACE))
            .class("foo")
            .class(["foo", "-webkit-foo", "-ms-foo"])
            .class_signal("foo", always(true))
            .class_signal(["foo", "-webkit-foo", "-ms-foo"], always(true));
    }

    #[test]
    fn style_signal_types() {
        lazy_static! {
            static ref FOO: String = "foo".to_owned();
        }

        let _a: DomBuilder<HtmlElement> = DomBuilder::new(create_element_ns("div", HTML_NAMESPACE))
            .style_signal("foo", always("bar"))
            .style_signal("foo", always("bar".to_owned()))
            .style_signal(
                "foo",
                always("bar".to_owned()).map(|x| RefFn::new(x, |x| x.as_str())),
            )
            .style("foo".to_owned(), "bar".to_owned())
            .style_signal("foo".to_owned(), always("bar".to_owned()))
            .style(&"foo".to_owned(), &"bar".to_owned())
            //.style(Box::new("foo".to_owned()), Box::new("bar".to_owned()))
            //.style_signal(Box::new("foo".to_owned()), always(Box::new("bar".to_owned())))
            .style_signal(&*FOO, always(&*FOO))
            //.style(vec!["-moz-foo", "-webkit-foo", "foo"].as_slice(), vec!["bar"].as_slice())
            .style_signal(
                RefFn::new(vec!["-moz-foo", "-webkit-foo", "foo"], |x| x.as_slice()),
                always(RefFn::new(vec!["bar"], |x| x.as_slice())),
            )
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always("bar"))
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always("bar".to_owned()))
            .style_signal(
                ["-moz-foo", "-webkit-foo", "foo"],
                always("bar".to_owned()).map(|x| RefFn::new(x, |x| x.as_str())),
            )
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always(["bar", "qux"]))
            .style_signal(
                ["-moz-foo", "-webkit-foo", "foo"],
                always(["bar".to_owned(), "qux".to_owned()]),
            )
            //.style_signal(["-moz-foo", "-webkit-foo", "foo"], always(AsSlice::new(["foo", "bar"])))
            //.style_signal(["-moz-foo", "-webkit-foo", "foo"], always(("bar".to_owned(), "qux".to_owned())).map(|x| RefFn::new(x, |x| AsSlice::new([x.0.as_str(), x.1.as_str()]))))
            .style_signal("foo", always(Some("bar")))
            .style_signal("foo", always(Some("bar".to_owned())))
            .style_signal(
                "foo",
                always("bar".to_owned()).map(|x| Some(RefFn::new(x, |x| x.as_str()))),
            )
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always(Some("bar")))
            .style_signal(
                ["-moz-foo", "-webkit-foo", "foo"],
                always(Some("bar".to_owned())),
            )
            .style_signal(
                ["-moz-foo", "-webkit-foo", "foo"],
                always("bar".to_owned()).map(|x| Some(RefFn::new(x, |x| x.as_str()))),
            );
    }
}

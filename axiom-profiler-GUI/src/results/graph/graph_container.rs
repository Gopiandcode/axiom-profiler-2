use petgraph::graph::{EdgeIndex, NodeIndex};
use smt_log_parser::items::InstIdx;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Element, HtmlInputElement};
use yew::prelude::*;

use crate::results::svg_result::RenderedGraph;
use crate::{CallbackRef, GlobalCallbacksContext, PagePosition, PrecisePosition};

use super::svg_graph::{Graph, Svg};

pub enum Msg {
    SetValueTo(f32),
    SetScrollTo((PrecisePosition, PrecisePosition)),
    Wheel(WheelEvent),
    Scroll(Event),
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp(MouseEvent),
}

pub struct GraphContainer {
    graph: Option<(Html, u32)>,
    mouse_closures: Option<Closure<dyn Fn(MouseEvent)>>,
    scroll_window: NodeRef,
    drag_start: Option<(PagePosition, PagePosition, bool)>,
    zoom_factor: f32,
    scroll_position: PrecisePosition,
    scroll_bound: PrecisePosition,
    zoom_factor_delta: f32,
    _callback_refs: [CallbackRef; 2],
}

impl GraphContainer {
    fn scroll_by(&mut self, x: f64, y: f64) {
        let pos = PrecisePosition { x: self.scroll_position.x + x, y: self.scroll_position.y + y };
        self.scroll_to(pos);
    }
    fn scroll_to(&mut self, pos: PrecisePosition) {
        self.scroll_position.x = pos.x.min(self.scroll_bound.x).max(0.0);
        self.scroll_position.y = pos.y.min(self.scroll_bound.y).max(0.0);
        self.scroll_window.cast::<Element>().unwrap_throw().scroll_to_with_x_and_y(self.scroll_position.x, self.scroll_position.y);
    }
}

#[derive(Properties, PartialEq)]
pub struct GraphContainerProps {
    pub rendered: Option<RenderedGraph>,
    pub update_selected_nodes: Callback<NodeIndex>,
    pub update_selected_edges: Callback<EdgeIndex>,
    pub deselect_all: Callback<()>,
    pub selected_nodes: Vec<NodeIndex>,
    pub selected_edges: Vec<EdgeIndex>,
}

impl Component for GraphContainer {
    type Message = Msg;
    type Properties = GraphContainerProps;

    fn create(ctx: &Context<Self>) -> Self {
        let graph = ctx.props().rendered.as_ref().map(|r| (Html::from_html_unchecked(r.svg_text.clone()), r.graph.generation));
        let registerer = ctx.link().get_callbacks_registerer().unwrap();
        let mouse_move_ref = (registerer.register_mouse_move)(ctx.link().callback(Msg::MouseMove));
        let mouse_up_ref = (registerer.register_mouse_up)(ctx.link().callback(Msg::MouseUp));
        let _callback_refs = [mouse_move_ref, mouse_up_ref];
        let scroll_position = PrecisePosition { x: 0.0, y: 0.0 };
        let scroll_bound = PrecisePosition { x: 0.0, y: 0.0 };
        Self { graph, mouse_closures: None, drag_start: None, scroll_window: NodeRef::default(), zoom_factor: 1.0, scroll_position, scroll_bound, zoom_factor_delta: 1.0, _callback_refs }
    }
    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        self.graph = ctx.props().rendered.as_ref().map(|r| (Html::from_html_unchecked(r.svg_text.clone()), r.graph.generation));
        self.zoom_factor_delta = 1.0;
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetValueTo(value) => {
                let zoom_factor = value.min(5.0).max(0.005);
                self.zoom_factor_delta = zoom_factor / self.zoom_factor;
                self.zoom_factor = zoom_factor;
                true
            }
            Msg::SetScrollTo((pos, bound)) => {
                self.scroll_bound = bound;
                self.scroll_to(pos);
                false
            }
            Msg::Wheel(ev) => {
                if ev.ctrl_key() {
                    ev.prevent_default();
                    let need_to_flip = ev.delta_y() >= 0.0;
                    // We need `delta < 0` such that the `powf` doesn't overflow
                    let delta = -ev.delta_y().abs() as f32 * SPREAD;
                    const SPREAD: f32 = 0.5;
                    let e_pow = std::f32::consts::E.powf(delta);
                    let sigmoid = (1.0 - e_pow) / (1.0 + e_pow);
                    const MAX_DEV: f32 = 0.1;
                    let factor = if need_to_flip {
                        1.0 - sigmoid * MAX_DEV
                    } else {
                        1.0 + sigmoid * MAX_DEV
                    };
                    let zoom_factor = (self.zoom_factor * factor).min(5.0).max(0.005);
                    self.zoom_factor_delta = zoom_factor / self.zoom_factor;
                    self.zoom_factor = zoom_factor;
                    true
                } else {
                    ev.prevent_default();
                    self.scroll_by(ev.delta_x(), ev.delta_y());
                    false
                }
            }
            Msg::Scroll(_) => {
                let scroll_window = self.scroll_window.cast::<Element>().unwrap_throw();
                self.scroll_position.x = scroll_window.scroll_left() as f64;
                self.scroll_position.y = scroll_window.scroll_top() as f64;
                false
            }
            Msg::MouseDown(ev) => {
                let pos = PagePosition { x: ev.client_x(), y: ev.client_y() };
                self.drag_start = Some((pos, pos, false));
                false
            }
            Msg::MouseMove(ev) => {
                if ev.buttons() != 1 {
                    self.drag_start = None;
                } else {
                    if let Some((start, last, drag)) = &mut self.drag_start {
                        let pos = PagePosition { x: ev.client_x(), y: ev.client_y() };
                        if (start.x - pos.x).abs() > 5 || (start.y - pos.y).abs() > 5 {
                            *drag = true;
                            let (dx, dy) = ((last.x - pos.x) as f64, (last.y - pos.y) as f64);
                            *last = pos;
                            self.scroll_by(dx, dy);
                        }
                    }
                }
                false
            }
            Msg::MouseUp(_) => {
                if let Some((_, _, drag)) = self.drag_start.take() {
                    if !drag {
                        ctx.props().deselect_all.emit(());
                    }
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let input = NodeRef::default();
        let set_value = {
            move |input_event: Event| {
                let target: HtmlInputElement = input_event
                    .target()
                    .unwrap_throw()
                    .dyn_into()
                    .unwrap_throw();
                match target.value().to_string().parse::<f32>() {
                    Ok(value) => {
                        // log::debug!("Setting the value to {}", value);
                        Msg::SetValueTo(value)
                    }
                    Err(_) => Msg::SetValueTo(1.0),
                }
            }
        };
        let link = ctx.link().clone();
        let input_ref = input.clone();
        let set_value_on_enter = Callback::from(move |key_event: KeyboardEvent| {
            if key_event.key() == "Enter" {
                let target = input_ref.cast::<HtmlInputElement>().unwrap_throw();
                let msg = match target.value().parse::<f32>() {
                    Ok(value) => Msg::SetValueTo(value),
                    Err(_) => Msg::SetValueTo(1.0),
                };
                link.send_message(msg);
            }
        });
        let set_value_on_blur = ctx.link().callback(move |blur_event: FocusEvent| {
            let event: Event = blur_event.clone().into();
            set_value(event)
        });
        let onwheel = ctx.link().callback(Msg::Wheel);
        let onscroll = ctx.link().callback(Msg::Scroll);
        let zoom_factor = format!("{:.3}", self.zoom_factor);
        let idx = zoom_factor.chars().rev().position(|c| c != '0' && c != '.').unwrap_or(zoom_factor.len() - 1);
        let zoom_factor = zoom_factor[0..zoom_factor.len() - idx].to_string();
        let set_scroll = ctx.link().callback(Msg::SetScrollTo);
        html! {
        <div ref={&self.scroll_window} style="height: 100%; overflow: auto; overscroll-behavior-x: none;" {onwheel} {onscroll}>
            <div style="position: absolute; bottom: 0; left: 0; z-index: 1;">
                <label for="input">{"Zoom factor: "}</label>
                <input ref={input} onkeypress={set_value_on_enter} onblur={set_value_on_blur} id="input" size="5" value={zoom_factor}/>
            </div>
            <Graph
                rendered={ctx.props().rendered.clone()}
                update_selected_nodes={&ctx.props().update_selected_nodes}
                update_selected_edges={&ctx.props().update_selected_edges}
                zoom_factor={self.zoom_factor}
                zoom_factor_delta={self.zoom_factor_delta}
                selected_nodes={ctx.props().selected_nodes.clone()}
                selected_edges={ctx.props().selected_edges.clone()}
                scroll_position={self.scroll_position.clone()}
                set_scroll={set_scroll}
                scroll_window={self.scroll_window.clone()}
            ><Svg svg={self.graph.clone()}/></Graph>
        </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let mouse_down = ctx.link().callback(Msg::MouseDown);
            let mouse_down: Closure<dyn Fn(MouseEvent)> = Closure::new(move |ev| mouse_down.emit(ev));
            // attach event listener to node
            let div = self.scroll_window.cast::<Element>().unwrap_throw();
            div.add_event_listener_with_callback(
                "mousedown",
                mouse_down.as_ref().unchecked_ref(),
            ).unwrap();
            self.mouse_closures = Some(mouse_down);
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        if let Some(mouse_down) = self.mouse_closures.take() {
            let div = self.scroll_window.cast::<Element>().unwrap_throw();
            div.remove_event_listener_with_callback(
                "mousedown",
                mouse_down.as_ref().unchecked_ref(),
            ).unwrap();
        }
    }
}

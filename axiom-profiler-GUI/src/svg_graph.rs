use yew::{prelude::*, virtual_dom::{VNode, VTag, VText}};
use scraper::{self, Selector};
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use crate::nodes::*;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let svg_text = use_state(|| String::new());
    let onclick = {
        let text = props.trace_file_text.to_string();
        let svg_text = svg_text.clone();
        Callback::from(move |_| {
            let text = text.to_string();
            let svg_text = svg_text.clone();
            log::debug!("use effect");
            wasm_bindgen_futures::spawn_local(
                async move {
                    let mut parser = z3parser1::Z3Parser1::new();
                    parser.process_log(text);
                    let qi_graph = parser.get_instantiation_graph();
                    let dot_output = format!("{:?}", Dot::with_config(qi_graph, &[Config::EdgeNoLabel])); 
                    let graphviz = VizInstance::new().await;
                    let svg = graphviz
                        .render_svg_element(dot_output, viz_js::Options::default())
                        .expect("Could not render graphviz");
                    let fetched_svg = svg.outer_html(); 
                    svg_text.set(fetched_svg);
                },
                   
            );
        })
    };
    html! {
        <>
            <div>
            <button onclick={onclick}>{"Load file"}</button>
            </div>
            <svg xmlns="http://www.w3.org/2000/svg" width="206pt" height="116pt" viewBox="0.00 0.00 206.00 116.00">
                <g id="graph0" class="graph" transform="scale (1 1) rotate(0) translate(4 112)">
                    <polygon fill="white" stroke="none" points="-4,4 -4,-112 202,-112 202,4 -4,4"></polygon>
                    <Nodes svg_text={(*svg_text).clone()} />
                </g>
            </svg>
        </>
    }
}


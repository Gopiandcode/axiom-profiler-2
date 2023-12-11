use gloo::console::log;
use web_sys::HtmlElement;
use yew::{prelude::*, virtual_dom::VNode};
use indexmap::map::IndexMap;
use petgraph::graph::{NodeIndex, EdgeIndex};
use smt_log_parser::{parsers::z3::inst_graph::{InstInfo, EdgeInfo}, items::DepType};
use smt_log_parser::parsers::z3::inst_graph::EdgeType;
use material_yew::WeakComponentLink;


pub struct InstsInfo {
    is_expanded_node: IndexMap<NodeIndex, bool>,
    selected_nodes_ref: NodeRef,
    is_expanded_edge: IndexMap<EdgeIndex, bool>, 
    selected_edges_ref: NodeRef,
}

pub enum Msg {
    AddNode(NodeIndex),
    RemoveNode(NodeIndex),
    ToggleOpenNode(NodeIndex),
    AddEdge(EdgeIndex),
    RemoveEdge(EdgeIndex),
    ToggleOpenEdge(EdgeIndex),
    AddNodes(Vec<NodeIndex>),
    RemoveAll,
}

#[derive(Properties, PartialEq)]
pub struct InstsInfoProps {
    pub selected_nodes: Vec<InstInfo>,
    // pub selected_edges: Vec<(NodeIndex, NodeIndex, EdgeInfo)>,
    pub selected_edges: Vec<EdgeInfo>,
    pub weak_link: WeakComponentLink<InstsInfo>,
}

impl Component for InstsInfo {
    type Message = Msg;

    type Properties = InstsInfoProps;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.props()
            .weak_link
            .borrow_mut()
            .replace(ctx.link().clone());
        Self {
            is_expanded_node: IndexMap::new(),
            selected_nodes_ref: NodeRef::default(),
            is_expanded_edge: IndexMap::new(),
            selected_edges_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddNode(node) => {
                // When adding a single new node to the vector,
                // close all  
                for val in self.is_expanded_node.values_mut() {
                    *val = false;
                }
                // except the added node
                self.is_expanded_node.insert(node, true);
                true
            },
            Msg::RemoveNode(node) => {
                self.is_expanded_node.remove(&node);
                true
            },
            Msg::ToggleOpenNode(node) => {
                let open_value = self.is_expanded_node.get_mut(&node).unwrap();
                *open_value = !*open_value;
                false
            },
            Msg::AddEdge(edge) => {
                for val in self.is_expanded_edge.values_mut() {
                    *val = false;
                }
                self.is_expanded_edge.insert(edge, true);
                true
            }
            Msg::RemoveEdge(edge) => {
                self.is_expanded_edge.remove(&edge);
                true
            },
            Msg::ToggleOpenEdge(edge) => {
                let open_value = self.is_expanded_edge.get_mut(&edge).unwrap();
                *open_value = !*open_value;
                false
            }
            Msg::RemoveAll => {
                self.is_expanded_node.clear();
                self.is_expanded_edge.clear();
                true
            },
            Msg::AddNodes(nodes) => {
                self.is_expanded_node.clear();
                for node in nodes {
                    self.is_expanded_node.insert(node, false);
                    log!(format!("Inserting node {} into is_expanded_node", node.index()));
                }
                true
            }
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        log!("Rendered details");
        let selected_nodes_details = self.selected_nodes_ref.cast::<HtmlElement>().expect("not attached to div element");
        let node_details = selected_nodes_details.get_elements_by_tag_name("details");
        for i in 0..node_details.length() {
            log!(format!("There are {} nodes", node_details.length()));
            let node_detail = node_details.item(i).unwrap();
            let node_id = node_detail.id().parse::<usize>().unwrap();
            log!(format!("node_details contains node {}", node_id));
            if *self.is_expanded_node.get(&NodeIndex::new(node_id)).unwrap() {
                let _ = node_detail.set_attribute("open", "true");
            } else {
                let _ = node_detail.remove_attribute("open");
            }
        }
        let selected_edges_details = self.selected_edges_ref.cast::<HtmlElement>().expect("not attached to div element");
        let edge_details = selected_edges_details.get_elements_by_tag_name("details");
        for i in 0..edge_details.length() {
            let edge_detail = edge_details.item(i).unwrap();
            let edge_id = edge_detail.id().parse::<usize>().unwrap();
            if *self.is_expanded_edge.get(&EdgeIndex::new(edge_id)).unwrap() {
                let _ = edge_detail.set_attribute("open", "true");
            } else {
                let _ = edge_detail.remove_attribute("open");
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // let selected_edges_info: Vec<VNode> = ctx.props()
        //     .selected_edges
        //     .iter()
        //     .map(|(from, to, data)| {
        //         html! {
        //         <details>
        //             <summary>{format!("Dependency from {} to {}", from.index(), to.index())}</summary>
        //             {match data.edge_data.edge_type {
        //                 EdgeType::Direct(DepType::Term) => html! {
        //                     <div>
        //                     <h4>{"Blame term: "}</h4><p>{data.blame_term.clone()}</p>
        //                     </div>
        //                 }, 
        //                 EdgeType::Direct(DepType::Equality) => html! {
        //                     <div>
        //                     <h4>{"Equality: "}</h4><p>{data.blame_term.clone()}</p>
        //                     </div>
        //                 },
        //                 _ => html! {} 
        //             }}
        //         </details>
        //         }
        //     })
        //     .collect();
        let on_node_click = {
            let link = ctx.link().clone();
            Callback::from(move |node: NodeIndex| {
                link.send_message(Msg::ToggleOpenNode(node))
            })
        };
        let on_edge_click = {
            let link = ctx.link().clone();
            Callback::from(move |edge: EdgeIndex| {
                link.send_message(Msg::ToggleOpenEdge(edge))
            })
        };
        html! {
            <>
                <h2>{"Information about selected nodes:"}</h2>
                <div ref={self.selected_nodes_ref.clone()}>
                    <SelectedNodesInfo selected_nodes={ctx.props().selected_nodes.clone()} on_click={on_node_click} />
                </div>
                <h2>{"Information about selected dependencies:"}</h2>
                <div ref={self.selected_edges_ref.clone()}>
                    <SelectedEdgesInfo selected_edges={ctx.props().selected_edges.clone()} on_click={on_edge_click} />
                </div>
            </>
        }
    }
}

#[derive(Properties, PartialEq)]
struct SelectedNodesInfoProps {
    selected_nodes: Vec<InstInfo>,
    on_click: Callback<NodeIndex>,
}

#[function_component(SelectedNodesInfo)]
fn selected_nodes_info(SelectedNodesInfoProps { selected_nodes, on_click }: &SelectedNodesInfoProps) -> Html {
    selected_nodes 
        .iter()
        .map(|selected_inst| { 
            let get_ul = |label: &str, items: &Vec<String>| html! {
                <>
                    <h4>{label}</h4>
                    <ul>{for items.iter().map(|item| html!{<li>{item}</li>})}</ul>
                </>
            };
            let on_select = {
                let on_click = on_click.clone();
                let selected_inst = selected_inst.clone();
                Callback::from(move |_| {
                    on_click.emit(selected_inst.node_index.clone())
                })
            };
            // let open = *self.is_expanded.get(&selected_inst.node_index.index()).unwrap(); 
            html! {
            <details id={format!("{}", selected_inst.node_index.index())} onclick={on_select}>
                <summary>{format!("Node {}", selected_inst.node_index.index())}</summary>
                <ul>
                    <li><h4>{"Instantiation happens at line number: "}</h4><p>{if let Some(val) = selected_inst.line_no {format!("{val}")} else { String::new() }}</p></li>
                    <li><h4>{"Cost: "}</h4><p>{selected_inst.cost}</p></li>
                    <li><h4>{"Instantiated formula: "}</h4><p>{&selected_inst.formula}</p></li>
                    <li>{get_ul("Blamed terms: ", &selected_inst.blamed_terms)}</li>
                    <li>{get_ul("Bound terms: ", &selected_inst.bound_terms)}</li>
                    <li>{get_ul("Yield terms: ", &selected_inst.yields_terms)}</li>
                    <li>{get_ul("Equality explanations: ", &selected_inst.equality_expls)}</li>
                    <li><h4>{"Resulting term: "}</h4><p>{if let Some(ref val) = selected_inst.resulting_term {format!("{val}")} else { String::new() }}</p></li>
                </ul>
            </details>
        }})
        .collect()
}

#[derive(Properties, PartialEq)]
struct SelectedEdgesInfoProps {
    selected_edges: Vec<EdgeInfo>,
    on_click: Callback<EdgeIndex>,
}

#[function_component(SelectedEdgesInfo)]
fn selected_edges_info(SelectedEdgesInfoProps { selected_edges, on_click }: &SelectedEdgesInfoProps) -> Html {
    selected_edges
        .iter()
        .map(|selected_edge| {
            let on_select = {
                let on_click = on_click.clone();
                let selected_edge = selected_edge.clone();
                Callback::from(move |_| {
                    on_click.emit(selected_edge.edge_data.orig_graph_idx.unwrap().clone())
                })
            };
            html! {
            <details id={format!("{}", selected_edge.edge_data.orig_graph_idx.unwrap().index())} onclick={on_select}>
                <summary>{format!("Dependency from {} to {}", selected_edge.from.index(), selected_edge.to.index())}</summary>
                {match selected_edge.edge_data.edge_type {
                    EdgeType::Direct(DepType::Term) => html! {
                        <div>
                        <h4>{"Blame term: "}</h4><p>{selected_edge.blame_term.clone()}</p>
                        </div>
                    }, 
                    EdgeType::Direct(DepType::Equality) => html! {
                        <div>
                        <h4>{"Equality: "}</h4><p>{selected_edge.blame_term.clone()}</p>
                        </div>
                    },
                    _ => html! {} 
                }}
            </details>
            }
        })
        .collect()
}
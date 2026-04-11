use leptos_node_graph::{TypedNodeDef, TypedPort};

use crate::DemoPort;

pub fn node_catalog() -> Vec<TypedNodeDef<DemoPort>> {
    vec![
        TypedNodeDef {
            id: "color_source".into(),
            label: "Color Source".into(),
            category: Some("Input".into()),
            description: Some("Produces a color and alpha".into()),
            ports: vec![
                TypedPort::output("color", "Color", DemoPort::Color),
                TypedPort::output("alpha", "Alpha", DemoPort::Float),
            ],
        },
        TypedNodeDef {
            id: "mix".into(),
            label: "Mix".into(),
            category: Some("Color".into()),
            description: Some("Blend two colors".into()),
            ports: vec![
                TypedPort::input("a", "A", DemoPort::Color),
                TypedPort::input("b", "B", DemoPort::Color),
                TypedPort::input("factor", "Factor", DemoPort::Float),
                TypedPort::output("result", "Result", DemoPort::Color),
            ],
        },
        TypedNodeDef {
            id: "math".into(),
            label: "Math".into(),
            category: Some("Math".into()),
            description: Some("Arithmetic operation".into()),
            ports: vec![
                TypedPort::input("a", "A", DemoPort::Float),
                TypedPort::input("b", "B", DemoPort::Float),
                TypedPort::output("result", "Result", DemoPort::Float),
            ],
        },
        TypedNodeDef {
            id: "output".into(),
            label: "Output".into(),
            category: Some("Output".into()),
            description: Some("Final output destination".into()),
            ports: vec![
                TypedPort::input("color", "Color", DemoPort::Color),
                TypedPort::input("value", "Value", DemoPort::Any),
            ],
        },
        TypedNodeDef {
            id: "custom".into(),
            label: "Custom".into(),
            category: Some("Utility".into()),
            description: Some("Configurable inputs/outputs".into()),
            ports: vec![
                TypedPort::input("in_0", "In 0", DemoPort::Any),
                TypedPort::output("out_0", "Out 0", DemoPort::Any),
            ],
        },
    ]
}

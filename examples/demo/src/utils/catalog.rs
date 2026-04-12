use leptos_node_graph::{Category, TypedNodeDef, TypedPort};

use crate::DemoPort;

pub fn node_catalog() -> Vec<TypedNodeDef<DemoPort>> {
    vec![
        TypedNodeDef {
            id: "color_source".into(),
            label: "Color Source".into(),
            category: Some(Category::with_color("Input", "#22d3ee")),
            description: Some("Produces a color and alpha".into()),
            ports: vec![
                TypedPort::output("color", "Color", DemoPort::Color),
                TypedPort::output("alpha", "Alpha", DemoPort::Float),
            ],
        },
        TypedNodeDef {
            id: "mix".into(),
            label: "Mix".into(),
            category: Some(Category::with_color("Color", "#f59e0b")),
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
            category: Some(Category::with_color("Math", "#8b5cf6")),
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
            category: Some(Category::with_color("Output", "#ef4444")),
            description: Some("Final output destination".into()),
            ports: vec![
                TypedPort::input("color", "Color", DemoPort::Color),
                TypedPort::input("value", "Value", DemoPort::Any),
            ],
        },
        TypedNodeDef {
            id: "custom".into(),
            label: "Custom".into(),
            category: Some(Category::with_color("Utility", "#10b981")),
            description: Some("Configurable inputs/outputs".into()),
            ports: vec![],
        },
    ]
}

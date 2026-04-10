use leptos_node_graph::{MenuPort, NodeMenuItem, PortDirection};

pub fn node_catalog() -> Vec<NodeMenuItem> {
    vec![
        NodeMenuItem {
            id: "color_source".into(),
            label: "Color Source".into(),
            category: Some("Input".into()),
            description: Some("Produces a color and alpha".into()),
            ports: vec![
                MenuPort {
                    id: "color".into(),
                    label: "Color".into(),
                    direction: PortDirection::Output,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "alpha".into(),
                    label: "Alpha".into(),
                    direction: PortDirection::Output,
                    type_id: "Float".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "mix".into(),
            label: "Mix".into(),
            category: Some("Color".into()),
            description: Some("Blend two colors".into()),
            ports: vec![
                MenuPort {
                    id: "a".into(),
                    label: "A".into(),
                    direction: PortDirection::Input,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "b".into(),
                    label: "B".into(),
                    direction: PortDirection::Input,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "factor".into(),
                    label: "Factor".into(),
                    direction: PortDirection::Input,
                    type_id: "Float".into(),
                },
                MenuPort {
                    id: "result".into(),
                    label: "Result".into(),
                    direction: PortDirection::Output,
                    type_id: "Color".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "math".into(),
            label: "Math".into(),
            category: Some("Math".into()),
            description: Some("Arithmetic operation".into()),
            ports: vec![
                MenuPort {
                    id: "a".into(),
                    label: "A".into(),
                    direction: PortDirection::Input,
                    type_id: "Float".into(),
                },
                MenuPort {
                    id: "b".into(),
                    label: "B".into(),
                    direction: PortDirection::Input,
                    type_id: "Float".into(),
                },
                MenuPort {
                    id: "result".into(),
                    label: "Result".into(),
                    direction: PortDirection::Output,
                    type_id: "Float".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "output".into(),
            label: "Output".into(),
            category: Some("Output".into()),
            description: Some("Final output destination".into()),
            ports: vec![
                MenuPort {
                    id: "color".into(),
                    label: "Color".into(),
                    direction: PortDirection::Input,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "value".into(),
                    label: "Value".into(),
                    direction: PortDirection::Input,
                    type_id: "Any".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "custom".into(),
            label: "Custom".into(),
            category: Some("Utility".into()),
            description: Some("Configurable inputs/outputs".into()),
            ports: vec![
                MenuPort {
                    id: "in_0".into(),
                    label: "In 0".into(),
                    direction: PortDirection::Input,
                    type_id: "Any".into(),
                },
                MenuPort {
                    id: "out_0".into(),
                    label: "Out 0".into(),
                    direction: PortDirection::Output,
                    type_id: "Any".into(),
                },
            ],
        },
    ]
}

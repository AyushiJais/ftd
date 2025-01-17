use crate::IText;

#[derive(serde::Deserialize, Debug, PartialEq, Default, Clone, serde::Serialize)]
pub struct Node {
    pub condition: Option<ftd::Condition>,
    pub events: Vec<ftd::Event>,
    pub classes: Vec<String>,
    pub node: String,
    pub attrs: ftd::Map,
    pub style: ftd::Map,
    pub children: Vec<Node>,
    pub external_children: Vec<Node>,
    pub open_id: Option<String>,
    pub external_children_container: Vec<Vec<usize>>,
    pub children_style: ftd::Map,
    pub text: Option<String>,
    pub null: bool,
}

impl Node {
    pub fn fixed_children_style(&self, index: usize) -> ftd::Map {
        if index == 1 {
            let mut list: ftd::Map = Default::default();
            for (key, value) in self.children_style.iter() {
                if key == "margin-left" || key == "margin-top" {
                    continue;
                }
                list.insert(key.clone(), value.clone());
            }
            list
        } else {
            self.children_style.clone()
        }
    }

    pub fn is_visible(&self, data: &ftd::DataDependenciesMap) -> bool {
        if self.null {
            return false;
        }

        match self.condition {
            Some(ref v) => v.is_true(data),
            None => true,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn to_dnode(
        &self,
        style: &ftd::Map,
        data: &ftd::DataDependenciesMap,
        external_children: &mut Option<Vec<Self>>,
        external_open_id: &Option<String>,
        external_children_container: &[Vec<usize>],
        is_parent_visible: bool,
        parent_id: &str,
        is_last: bool,
    ) -> ftd::dnode::DNode {
        let style = {
            let mut s = self.style.clone();
            s.extend(style.clone());
            s
        };

        let all_children = {
            let mut children: Vec<ftd::Node> = self.children.to_vec();
            #[allow(clippy::blocks_in_if_conditions)]
            if let Some(ext_children) = external_children {
                if *external_open_id
                    == self.attrs.get("data-id").map(|v| {
                        if v.contains(':') {
                            let mut part = v.splitn(2, ':');
                            part.next().unwrap().trim().to_string()
                        } else {
                            v.to_string()
                        }
                    })
                    && self.open_id.is_none()
                    && external_children_container.is_empty()
                    && ((self.is_visible(data) && is_parent_visible) || is_last)
                {
                    for child in ext_children.iter() {
                        if let Some(data_id) = child.attrs.get("data-id") {
                            for child in child.children.iter() {
                                let mut child = child.clone();
                                child.attrs.insert(
                                    "data-ext-id".to_string(),
                                    format!("{}:{}", data_id, parent_id),
                                );
                                children.push(child);
                            }
                        }
                    }
                    *external_children = None;
                }
            }
            children
        };

        let (open_id, external_children_container) =
            if self.open_id.is_some() && external_children_container.is_empty() {
                (&self.open_id, self.external_children_container.as_slice())
            } else {
                (external_open_id, external_children_container)
            };

        let mut ext_child = None;
        let mut is_borrowed_ext_child = false;

        let ext_child: &mut Option<Vec<Self>> = {
            if external_children_container.is_empty() {
                &mut ext_child
            } else if self.open_id.is_some() && !self.external_children.is_empty() {
                ext_child = Some(self.external_children.clone());
                &mut ext_child
            } else {
                is_borrowed_ext_child = true;
                external_children
            }
        };

        let mut index = 0;
        let mut index_of_visible_children = 0;

        let children = {
            let mut children: Vec<ftd::dnode::DNode> = vec![];
            for (i, v) in all_children.iter().enumerate() {
                if v.node.is_empty() {
                    continue;
                }

                let (external_container, is_last) = {
                    let mut external_container = vec![];
                    while index < external_children_container.len() {
                        if let Some(container) = external_children_container[index].get(0) {
                            if container < &i {
                                index += 1;
                                continue;
                            }
                            let external_child_container =
                                external_children_container[index][1..].to_vec();
                            if container == &i {
                                if !external_child_container.is_empty() {
                                    external_container.push(external_child_container)
                                }
                            } else {
                                break;
                            }
                        }
                        index += 1;
                    }
                    let is_last = {
                        let mut last = external_container.is_empty()
                            && (index >= external_children_container.len()
                                || !is_other_sibling_visible(
                                    i,
                                    &all_children,
                                    index,
                                    external_children_container,
                                ));
                        if is_borrowed_ext_child {
                            last = is_last && last;
                        }
                        last
                    };

                    (external_container, is_last)
                };

                if v.is_visible(data) {
                    index_of_visible_children += 1;
                }

                children.push(v.to_dnode(
                    &self.fixed_children_style(index_of_visible_children),
                    data,
                    ext_child,
                    open_id,
                    external_container.as_slice(),
                    is_parent_visible && self.is_visible(data),
                    parent_id,
                    is_last,
                ));
            }
            children
        };

        let attrs = {
            let mut attrs = self.attrs.to_owned();
            let oid = if let Some(oid) = attrs.get("data-id") {
                format!("{}:{}", oid, parent_id)
            } else {
                format!("{}:root", parent_id)
            };
            attrs.insert("data-id".to_string(), oid);
            attrs
        };

        return ftd::dnode::DNode {
            classes: self.classes.to_owned(),
            node: self.node.to_owned(),
            attrs,
            style,
            children,
            text: self.text.to_owned(),
            null: self.null.to_owned(),
            events: self.events.to_owned(),
            visible: self.is_visible(data),
        };

        fn is_other_sibling_visible(
            index: usize,
            all_children: &[Node],
            ext_child_container_index: usize,
            external_children_container: &[Vec<usize>],
        ) -> bool {
            for external_child_container in external_children_container
                .iter()
                .skip(ext_child_container_index)
            {
                if let Some(container) = external_child_container.get(0) {
                    if container < &index {
                        continue;
                    }
                    if let Some(child) = all_children.get(*container) {
                        if !child.node.is_empty() {
                            return true;
                        }
                    }
                }
            }
            false
        }
    }

    pub fn to_html(&self, style: &ftd::Map, data: &ftd::DataDependenciesMap, id: &str) -> String {
        self.to_dnode(style, data, &mut None, &None, &[], true, id, false)
            .to_html(id)
    }

    pub fn get_target_node(&mut self, container: Vec<usize>) -> &mut Self {
        let mut current = self;
        for i in container.iter() {
            current = &mut current.children[*i];
        }
        current
    }
}

impl ftd::Element {
    pub fn to_node(&self, doc_id: &str) -> Node {
        match self {
            Self::Row(i) => (i.to_node(doc_id)),
            Self::Scene(i) => (i.to_node(doc_id)),
            Self::Grid(i) => (i.to_node(doc_id)),
            Self::Markup(i) => (i.to_node(doc_id)),
            Self::Text(i) => (i.to_node(doc_id)),
            Self::TextBlock(i) => (i.to_node(doc_id)),
            Self::Code(i) => (i.to_node(doc_id)),
            Self::Image(i) => (i.to_node(doc_id)),
            Self::Column(i) => (i.to_node(doc_id)),
            Self::IFrame(i) => (i.to_node(doc_id)),
            Self::Input(i) => (i.to_node(doc_id)),
            Self::Integer(i) => (i.to_node(doc_id)),
            Self::Boolean(i) => (i.to_node(doc_id)),
            Self::Decimal(i) => (i.to_node(doc_id)),
            Self::Null => Node {
                condition: None,
                events: vec![],
                classes: vec![],
                node: "".to_string(),
                attrs: Default::default(),
                style: Default::default(),
                children: vec![],
                external_children: Default::default(),
                open_id: None,
                external_children_container: vec![],
                children_style: Default::default(),
                text: None,
                null: true,
            },
        }
    }

    // TODO: only when wasm feature is enabled
    pub fn to_dom(&self, _id: &str) {
        todo!()
    }
}

impl Node {
    fn from_common(node: &str, common: &ftd::Common, doc_id: &str) -> Self {
        Node {
            condition: common.condition.clone(),
            node: s(node),
            attrs: common.attrs(),
            style: common.style(doc_id),
            children: vec![],
            external_children: Default::default(),
            open_id: None,
            external_children_container: vec![],
            children_style: common.children_style(),
            text: None,
            classes: common.add_class(),
            null: common.is_dummy,
            events: common.events.clone(),
        }
    }

    fn from_container(common: &ftd::Common, container: &ftd::Container, doc_id: &str) -> Self {
        let mut attrs = common.attrs();
        attrs.extend(container.attrs());
        let mut style = common.style(doc_id);
        style.extend(container.style());
        let mut classes = common.add_class();
        classes.extend(container.add_class());

        let mut children_style = common.children_style();
        children_style.extend(container.children_style());
        let node = match common.link {
            Some(_) => "a",
            None => match common.submit {
                Some(_) => "form",
                None => "div",
            },
        };

        let (id, external_children_container, external_children) = {
            if let Some((id, external_children_container, child)) = &container.external_children {
                (
                    Some(id.to_string()),
                    external_children_container.clone(),
                    child.iter().map(|v| v.to_node(doc_id)).collect(),
                )
            } else {
                (None, vec![], vec![])
            }
        };

        Node {
            condition: common.condition.clone(),
            node: s(node), // TODO: use better tags based on common.region
            attrs,
            style,
            classes,
            children_style,
            text: None,
            children: container
                .children
                .iter()
                .map(|v| v.to_node(doc_id))
                .collect(),
            external_children,
            open_id: id,
            external_children_container,
            null: common.is_dummy,
            events: common.events.clone(),
        }
    }
}

impl ftd::Scene {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let node = {
            let mut node = Node {
                node: s("div"),
                ..Default::default()
            };
            if let Some(ref data_id) = self.common.data_id {
                node.attrs
                    .insert(s("data-id"), format!("{}:scene", data_id));
            } else {
                node.attrs.insert(s("data-id"), s("scene:root"));
            }
            node.style.insert(s("position"), s("relative"));
            let children = {
                let parent = {
                    let mut node = if let Some(ref img) = self.common.background_image {
                        let mut n = Node {
                            node: s("img"),
                            ..Default::default()
                        };
                        n.attrs.insert(s("src"), s(img));
                        n.attrs.insert(s("alt"), escape("Scene"));
                        n
                    } else {
                        Node {
                            node: s("div"),
                            ..Default::default()
                        }
                    };
                    node.style.insert(s("width"), s("100%"));
                    if !self.common.is_not_visible {
                        node.style.insert(s("display"), s("block"));
                    }
                    if let Some(p) = &self.common.height {
                        let (key, value) = length(p, "height");
                        node.style.insert(s(key.as_str()), value);
                    } else {
                        node.style.insert(s("height"), s("auto"));
                    }
                    if let Some(ref data_id) = self.common.data_id {
                        node.attrs
                            .insert(s("data-id"), format!("{}:scene-bg", data_id));
                    }
                    node
                };
                let mut children: Vec<Node> = self
                    .container
                    .children
                    .iter()
                    .map(|v| {
                        let mut n = v.to_node(doc_id);
                        n.style.insert(s("position"), s("absolute"));
                        n
                    })
                    .collect();
                children.insert(0, parent);
                children
            };

            let (id, external_children_container, external_children) = {
                if let Some((id, external_children_container, child)) =
                    &self.container.external_children
                {
                    (
                        Some(id.to_string()),
                        external_children_container.clone(),
                        child
                            .iter()
                            .map(|v| {
                                let mut n = v.to_node(doc_id);
                                n.style.insert(s("position"), s("absolute"));
                                n
                            })
                            .collect(),
                    )
                } else {
                    (None, vec![], vec![])
                }
            };

            node.children = children;
            node.open_id = id;
            node.external_children = external_children;
            node.external_children_container = external_children_container;
            node
        };

        let mut main_node = Node::from_common("div", &self.common, doc_id);
        if self.common.width.is_none() {
            main_node.style.insert(s("width"), s("1000px"));
        }
        if let Some(ref p) = self.spacing {
            let (key, value) = spacing(p, "margin-left");
            match p {
                ftd::Spacing::Absolute { value } => {
                    main_node.children_style.insert(key, format!("{}px", value));
                    main_node
                        .attrs
                        .insert(s("data-spacing"), format!("margin-left:{}px", value))
                }
                _ => main_node.style.insert(key, value),
            };
        }
        main_node.children = vec![node];
        main_node
    }
}

impl ftd::Grid {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let mut n = Node::from_container(&self.common, &self.container, doc_id);
        if self.inline {
            n.style.insert(s("display"), s("inline-grid"));
        } else {
            n.style.insert(s("display"), s("grid"));
        }
        let areas = self
            .slots
            .split('|')
            .map(|v| v.trim())
            .collect::<Vec<&str>>();
        let mut css_areas = s("");
        for area in areas {
            css_areas = format!("{}'{}'", css_areas, area);
        }
        n.style.insert(s("grid-template-areas"), css_areas);

        if let Some(ref columns) = self.slot_widths {
            n.style
                .insert(s("grid-template-columns"), s(columns.trim()));
        }
        if let Some(ref rows) = self.slot_heights {
            n.style.insert(s("grid-template-rows"), s(rows.trim()));
        }
        if let Some(ref gap) = self.spacing {
            n.style.insert(s("grid-gap"), format!("{}px", gap));
        }
        if let Some(ref gap) = self.spacing_vertical {
            n.style.insert(s("column-gap"), format!("{}px", gap));
        }
        if let Some(ref gap) = self.spacing_horizontal {
            n.style.insert(s("row-gap"), format!("{}px", gap));
        }
        if let Some(ref auto_flow) = self.auto_flow {
            n.style.insert(s("grid-auto-flow"), s(auto_flow));
        }

        n.children = {
            let mut children = vec![];
            for child in self.container.children.iter() {
                let mut child_node = child.to_node(doc_id);
                let common = if let Some(common) = child.get_common() {
                    common
                } else {
                    children.push(child_node);
                    continue;
                };
                if common.anchor.is_some() {
                    children.push(child_node);
                    continue;
                }
                if let Some(ref position) = common.position {
                    for (key, value) in grid_align(position) {
                        child_node.style.insert(s(key.as_str()), value);
                    }
                }
                children.push(child_node);
            }
            children
        };

        n
    }
}

impl ftd::Row {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let mut n = Node::from_container(&self.common, &self.container, doc_id);
        if !self.common.is_not_visible {
            n.style.insert(s("display"), s("flex"));
        }
        n.style.insert(s("flex-direction"), s("row"));
        if self.container.wrap {
            n.style.insert(s("flex-wrap"), s("wrap"));
        } else {
            n.style.insert(s("flex-wrap"), s("nowrap"));
        }

        n.style.insert(s("align-items"), s("flex-start"));

        n.style.insert(s("justify-content"), s("flex-start"));

        if let Some(ref p) = self.spacing {
            let (key, value) = spacing(p, "margin-left");
            match p {
                ftd::Spacing::Absolute { value } => {
                    n.children_style.insert(key, format!("{}px", value));
                    n.attrs
                        .insert(s("data-spacing"), format!("margin-left:{}px", value))
                }
                _ => n.style.insert(key, value),
            };
        }

        n.children = {
            let mut children = vec![];
            for child in self.container.children.iter() {
                let mut child_node = child.to_node(doc_id);
                let common = if let Some(common) = child.get_common() {
                    common
                } else {
                    children.push(child_node);
                    continue;
                };
                if common.anchor.is_some() {
                    children.push(child_node);
                    continue;
                }
                if let Some(ref position) = common.position {
                    for (key, value) in row_align(position) {
                        child_node.style.insert(s(key.as_str()), value);
                    }
                }
                children.push(child_node);
            }
            children
        };

        n
    }
}

impl ftd::Column {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let mut n = Node::from_container(&self.common, &self.container, doc_id);
        if !self.common.is_not_visible {
            n.style.insert(s("display"), s("flex"));
        }
        n.style.insert(s("flex-direction"), s("column"));
        if self.container.wrap {
            n.style.insert(s("flex-wrap"), s("wrap"));
        } else {
            n.style.insert(s("flex-wrap"), s("nowrap"));
        }
        n.style.insert(s("align-items"), s("flex-start"));

        n.style.insert(s("justify-content"), s("flex-start"));

        if let Some(ref p) = self.spacing {
            let (key, value) = spacing(p, "margin-top");
            match p {
                ftd::Spacing::Absolute { value } => {
                    n.children_style.insert(key, format!("{}px", value));
                    n.attrs
                        .insert(s("data-spacing"), format!("margin-top:{}px", value))
                }
                _ => n.style.insert(key, value),
            };
        }

        n.children = {
            let mut children = vec![];
            for child in self.container.children.iter() {
                let mut child_node = child.to_node(doc_id);
                let common = if let Some(common) = child.get_common() {
                    common
                } else {
                    children.push(child_node);
                    continue;
                };
                if common.anchor.is_some() {
                    children.push(child_node);
                    continue;
                }
                if let Some(ref position) = common.position {
                    for (key, value) in column_align(position) {
                        child_node.style.insert(s(key.as_str()), value);
                    }
                }
                children.push(child_node);
            }
            children
        };

        n
    }
}

impl ftd::Text {
    pub fn to_node(&self, doc_id: &str) -> Node {
        // TODO: proper tag based on self.common.region
        // TODO: if format is not markdown use pre
        let node = match &self.common.link {
            Some(_) => "a",
            None => match &self.common.submit {
                Some(_) => "form",
                _ => "div",
            },
        };
        let mut n = Node::from_common(node, &self.common, doc_id);
        n.text = Some(self.text.rendered.clone());
        let (key, value) = text_align(&self.text_align);
        n.style.insert(s(key.as_str()), value);
        if !self.line && self.font.is_none() {
            n.style.insert(s("line-height"), s("26px"));
        }

        if let Some(ref font) = self.font {
            n.style.insert(s("font-family"), font.font.to_string());
            n.style
                .insert(s("line-height"), format!("{}px", font.desktop.line_height));
            n.style
                .insert(s("font-size"), format!("{}px", font.desktop.size));
            n.style.insert(s("font-weight"), font.weight.to_string());
            if font.style.italic {
                n.style.insert(s("font-style"), s("italic"));
            }
            if font.style.underline {
                n.style.insert(s("text-decoration"), s("underline"));
            }
            if font.style.strike {
                n.style.insert(s("text-decoration"), s("line-through"));
            }
        }

        if self.style.italic {
            n.style.insert(s("font-style"), s("italic"));
        }
        if self.style.underline {
            n.style.insert(s("text-decoration"), s("underline"));
        }
        if self.style.strike {
            n.style.insert(s("text-decoration"), s("line-through"));
        }

        if let Some(p) = &self.line_clamp {
            n.style.insert(s("display"), "-webkit-box".to_string());
            n.style.insert(s("overflow"), "hidden".to_string());
            n.style.insert(s("-webkit-line-clamp"), format!("{}", p));
            n.style
                .insert(s("-webkit-box-orient"), "vertical".to_string());
        }

        // TODO: text styles
        n
    }
}

impl ftd::TextBlock {
    pub fn to_node(&self, doc_id: &str) -> Node {
        // TODO: proper tag based on self.common.region
        // TODO: if format is not markdown use pre
        let node = match &self.common.link {
            Some(_) => "a",
            None => match &self.common.submit {
                Some(_) => "form",
                _ => "div",
            },
        };
        let mut n = Node::from_common(node, &self.common, doc_id);
        n.text = Some(self.text.rendered.clone());
        let (key, value) = text_align(&self.text_align);
        n.style.insert(s(key.as_str()), value);
        if let Some(p) = self.size {
            n.style.insert(s("font-size"), format!("{}px", p));
        }
        if let Some(p) = self.line_height {
            n.style.insert(s("line-height"), format!("{}px", p));
        } else if !&self.line {
            n.style.insert(s("line-height"), s("26px"));
        }

        if !self.font.is_empty() {
            let family = self
                .font
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            n.style.insert(s("font-family"), family);
        }

        if self.style.italic {
            n.style.insert(s("font-style"), s("italic"));
        }
        if self.style.underline {
            n.style.insert(s("text-decoration"), s("underline"));
        }
        if self.style.strike {
            n.style.insert(s("text-decoration"), s("line-through"));
        }

        if let Some(p) = &self.line_clamp {
            n.style.insert(s("display"), "-webkit-box".to_string());
            n.style.insert(s("overflow"), "hidden".to_string());
            n.style.insert(s("-webkit-line-clamp"), format!("{}", p));
            n.style
                .insert(s("-webkit-box-orient"), "vertical".to_string());
        }

        // TODO: text styles
        n
    }
}

impl ftd::Code {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let node = match &self.common.link {
            Some(_) => "a",
            None => match &self.common.submit {
                Some(_) => "form",
                _ => "div",
            },
        };
        let mut n = Node::from_common(node, &self.common, doc_id);
        n.text = Some(self.text.rendered.clone());
        let (key, value) = text_align(&self.text_align);
        n.style.insert(s(key.as_str()), value);

        if self.font.is_none() {
            n.style.insert(s("line-height"), s("26px"));
        }

        if let Some(ref font) = self.font {
            n.style.insert(s("font-family"), font.font.to_string());
            n.style
                .insert(s("line-height"), format!("{}px", font.desktop.line_height));
            n.style
                .insert(s("font-size"), format!("{}px", font.desktop.size));
            n.style.insert(s("font-weight"), font.weight.to_string());
            if font.style.italic {
                n.style.insert(s("font-style"), s("italic"));
            }
            if font.style.underline {
                n.style.insert(s("text-decoration"), s("underline"));
            }
            if font.style.strike {
                n.style.insert(s("text-decoration"), s("line-through"));
            }
        }

        if self.style.italic {
            n.style.insert(s("font-style"), s("italic"));
        }
        if self.style.underline {
            n.style.insert(s("text-decoration"), s("underline"));
        }
        if self.style.strike {
            n.style.insert(s("text-decoration"), s("line-through"));
        }

        if let Some(p) = &self.line_clamp {
            n.style.insert(s("display"), "-webkit-box".to_string());
            n.style.insert(s("overflow"), "hidden".to_string());
            n.style.insert(s("-webkit-line-clamp"), format!("{}", p));
            n.style
                .insert(s("-webkit-box-orient"), "vertical".to_string());
        }

        n
    }
}

impl ftd::Image {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let mut n = Node::from_common("img", &self.common, doc_id);
        if self.common.link.is_some() {
            n.node = s("a");
            let mut img = Node {
                condition: None,
                events: vec![],
                classes: vec![],
                node: s("img"),
                attrs: Default::default(),
                style: Default::default(),
                children: vec![],
                external_children: vec![],
                open_id: None,
                external_children_container: vec![],
                children_style: Default::default(),
                text: None,
                null: false,
            };
            img.style.insert(s("width"), s("100%"));
            img.attrs.insert(s("src"), escape(self.src.light.as_str()));
            img.attrs
                .insert(s("alt"), escape(self.description.as_str()));
            if self.crop {
                img.style.insert(s("object-fit"), s("cover"));
                img.style.insert(s("object-position"), s("0 0"));
            }
            n.children.push(img);
        } else {
            n.attrs.insert(s("src"), escape(self.src.light.as_str()));
            n.attrs.insert(s("alt"), escape(self.description.as_str()));
            if self.crop {
                n.style.insert(s("object-fit"), s("cover"));
                n.style.insert(s("object-position"), s("0 0"));
                if self.common.width.is_none() {
                    n.style.insert(s("width"), s("100%"));
                }
            }
        }

        n
    }
}

impl ftd::IFrame {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let mut n = Node::from_common("iframe", &self.common, doc_id);
        n.attrs.insert(s("src"), escape(self.src.as_str()));
        n.attrs.insert(s("allow"), s("fullscreen"));
        n.attrs.insert(s("allowfullscreen"), s("allowfullscreen"));
        n
    }
}

impl ftd::Markups {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let node = match &self.common.link {
            Some(_) => "a",
            None => match &self.common.submit {
                Some(_) => "form",
                _ => "div",
            },
        };
        let mut n = Node::from_common(node, &self.common, doc_id);
        let (key, value) = text_align(&self.text_align);
        n.style.insert(s(key.as_str()), value);

        if !self.line && self.font.is_none() {
            n.style.insert(s("line-height"), s("26px"));
        }

        if let Some(ref font) = self.font {
            n.style.insert(s("font-family"), font.font.to_string());
            n.style
                .insert(s("line-height"), format!("{}px", font.desktop.line_height));
            n.style
                .insert(s("font-size"), format!("{}px", font.desktop.size));
            n.style.insert(s("font-weight"), font.weight.to_string());
            if font.style.italic {
                n.style.insert(s("font-style"), s("italic"));
            }
            if font.style.underline {
                n.style.insert(s("text-decoration"), s("underline"));
            }
            if font.style.strike {
                n.style.insert(s("text-decoration"), s("line-through"));
            }
        }

        if self.style.italic {
            n.style.insert(s("font-style"), s("italic"));
        }
        if self.style.underline {
            n.style.insert(s("text-decoration"), s("underline"));
        }
        if self.style.strike {
            n.style.insert(s("text-decoration"), s("line-through"));
        }

        if let Some(p) = &self.line_clamp {
            n.style.insert(s("display"), "-webkit-box".to_string());
            n.style.insert(s("overflow"), "hidden".to_string());
            n.style.insert(s("-webkit-line-clamp"), format!("{}", p));
            n.style
                .insert(s("-webkit-box-orient"), "vertical".to_string());
        }

        if self.children.is_empty() {
            n.text = Some(self.text.rendered.clone());
        }
        n.children = self
            .children
            .iter()
            .map(|v| v.to_node(doc_id, !self.line))
            .collect();
        n
    }
}

impl ftd::Markup {
    pub fn to_node(&self, doc_id: &str, is_paragraph: bool) -> Node {
        let mut n = match self.itext {
            ftd::IText::Text(ref t)
            | ftd::IText::Integer(ref t)
            | ftd::IText::Boolean(ref t)
            | ftd::IText::Decimal(ref t) => t.to_node(doc_id),
            ftd::IText::TextBlock(ref t) => t.to_node(doc_id),
            IText::Markup(ref t) => t.to_node(doc_id),
        };
        if n.node.eq("div") {
            if is_paragraph {
                n.node = s("p");
            } else {
                n.node = s("span");
            }
        }
        if self.children.is_empty() {
            return n;
        } else {
            n.text = None;
        }
        n.children = self
            .children
            .iter()
            .map(|v| v.to_node(doc_id, false))
            .collect();
        n
    }
}

impl ftd::Input {
    pub fn to_node(&self, doc_id: &str) -> Node {
        let mut n = Node::from_common("input", &self.common, doc_id);
        if let Some(ref p) = self.placeholder {
            n.attrs.insert(s("placeholder"), escape(p));
        }
        n
    }
}

impl ftd::Common {
    fn add_class(&self) -> Vec<String> {
        let d: Vec<String> = vec![s("ft_md")];
        d
    }
    fn children_style(&self) -> ftd::Map {
        let d: ftd::Map = Default::default();
        d
    }

    fn style(&self, doc_id: &str) -> ftd::Map {
        let mut d: ftd::Map = Default::default();

        if !self.events.is_empty() && self.cursor.is_none() {
            d.insert(s("cursor"), s("pointer"));
        }
        if self.is_not_visible {
            d.insert(s("display"), s("none"));
        }

        if let Some(p) = self.padding {
            d.insert(s("padding"), format!("{}px", p));
        }
        if let Some(p) = self.padding_left {
            d.insert(s("padding-left"), format!("{}px", p));
        }
        if let Some(ref cursor) = self.cursor {
            d.insert(s("cursor"), s(cursor));
        }
        if let Some(p) = self.padding_vertical {
            d.insert(s("padding-top"), format!("{}px", p));
            d.insert(s("padding-bottom"), format!("{}px", p));
        }
        if let Some(p) = self.padding_horizontal {
            d.insert(s("padding-left"), format!("{}px", p));
            d.insert(s("padding-right"), format!("{}px", p));
        }
        if let Some(p) = self.padding_right {
            d.insert(s("padding-right"), format!("{}px", p));
        }
        if let Some(p) = self.padding_top {
            d.insert(s("padding-top"), format!("{}px", p));
        }
        if let Some(p) = self.padding_bottom {
            d.insert(s("padding-bottom"), format!("{}px", p));
        }

        if let Some(p) = self.border_top_radius {
            d.insert(s("border-top-left-radius"), format!("{}px !important", p));
            d.insert(s("border-top-right-radius"), format!("{}px !important", p));
        }

        if let Some(p) = self.border_left_radius {
            d.insert(s("border-top-left-radius"), format!("{}px !important", p));
            d.insert(
                s("border-bottom-left-radius"),
                format!("{}px !important", p),
            );
        }

        if let Some(p) = self.border_right_radius {
            d.insert(s("border-top-right-radius"), format!("{}px !important", p));
            d.insert(
                s("border-bottom-right-radius"),
                format!("{}px !important", p),
            );
        }

        if let Some(p) = self.border_bottom_radius {
            d.insert(
                s("border-bottom-right-radius"),
                format!("{}px !important", p),
            );
            d.insert(
                s("border-bottom-left-radius"),
                format!("{}px !important", p),
            );
        }

        if let Some(p) = &self.width {
            let (key, value) = length(p, "width");
            d.insert(s(key.as_str()), value);
        }
        if let Some(p) = &self.min_width {
            let (key, value) = length(p, "min-width");
            d.insert(s(key.as_str()), value);
        }
        if let Some(p) = &self.max_width {
            let (key, value) = length(p, "max-width");
            d.insert(s(key.as_str()), value);
        }
        if let Some(p) = &self.height {
            let (key, value) = length(p, "height");
            d.insert(s(key.as_str()), value);
        }
        if let Some(p) = &self.min_height {
            let (key, value) = length(p, "min-height");
            d.insert(s(key.as_str()), value);
        }
        if let Some(p) = &self.max_height {
            let (key, value) = length(p, "max-height");
            d.insert(s(key.as_str()), value);
        }
        if let Some(p) = self.border_left {
            d.insert(s("border-left-width"), format!("{}px !important", p));
        }
        if let Some(p) = self.border_right {
            d.insert(s("border-right-width"), format!("{}px !important", p));
        }
        if let Some(p) = self.border_top {
            d.insert(s("border-top-width"), format!("{}px !important", p));
        }
        if let Some(p) = self.border_bottom {
            d.insert(s("border-bottom-width"), format!("{}px !important", p));
        }
        if let Some(p) = self.margin_left {
            d.insert(s("margin-left"), format!("{}px", p));
        }
        if let Some(p) = self.margin_right {
            d.insert(s("margin-right"), format!("{}px", p));
        }
        if let Some(p) = self.margin_top {
            d.insert(s("margin-top"), format!("{}px", p));
        }
        if let Some(p) = self.margin_bottom {
            d.insert(s("margin-bottom"), format!("{}px", p));
        }
        if let Some(p) = &self.background_color {
            d.insert(s("background-color"), color(&p.light));
        }
        if let Some(p) = &self.color {
            d.insert(s("color"), color(&p.light));
        }
        if let Some(p) = &self.border_color {
            d.insert(s("border-color"), color(&p.light));
        }
        if let Some(p) = &self.overflow_x {
            let (key, value) = overflow(p, "overflow-x");
            d.insert(s(key.as_str()), value);
        }
        if let Some(p) = &self.overflow_y {
            let (key, value) = overflow(p, "overflow-y");
            d.insert(s(key.as_str()), value);
        }
        if self.sticky {
            d.insert(s("position"), s("sticky"));
        }
        if let Some(p) = &self.top {
            d.insert(s("top"), format!("{}px", p));
        }
        if let Some(p) = &self.bottom {
            d.insert(s("bottom"), format!("{}px", p));
        }
        if let Some(p) = &self.left {
            d.insert(s("left"), format!("{}px", p));
        }
        if let Some(p) = &self.right {
            d.insert(s("right"), format!("{}px", p));
        }
        if self.submit.is_some() {
            d.insert(s("cursor"), s("pointer"));
        }
        if self.link.is_some() {
            d.insert(s("cursor"), s("pointer"));
        }
        if let Some(p) = &self.z_index {
            d.insert(s("z-index"), p.to_string());
        }
        if let Some(p) = &self.slot {
            d.insert(s("grid-area"), s(p));
        }
        if let Some(p) = &self.grid_column {
            d.insert(s("grid-column"), s(p));
        }
        if let Some(p) = &self.grid_row {
            d.insert(s("grid-row"), s(p));
        }
        if let Some(p) = &self.text_transform {
            d.insert(s("text-transform"), s(p));
        }
        if self.shadow_size.is_some()
            || self.shadow_blur.is_some()
            || self.shadow_offset_x.is_some()
            || self.shadow_offset_y.is_some()
        {
            let shadow_color = match &self.shadow_color {
                Some(p) => &p.light,
                None => &ftd::ColorValue {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 0.25,
                },
            };

            d.insert(
                s("box-shadow"),
                format!(
                    "{}px {}px {}px {}px {}",
                    self.shadow_offset_x.unwrap_or(0),
                    self.shadow_offset_y.unwrap_or(0),
                    self.shadow_blur.unwrap_or(0),
                    self.shadow_size.unwrap_or(0),
                    color(shadow_color),
                ),
            );
        }
        if let Some(p) = &self.anchor {
            d.insert(s("position"), p.to_position());
        }
        if let Some(p) = &self.gradient_direction {
            d.insert(s("background-image"), gradient(p, &self.gradient_colors));
        }
        if let Some(p) = &self.background_image {
            d.insert(s("background-image"), format!("url({})", p));
            if self.background_repeat {
                d.insert(s("background-repeat"), s("repeat"));
            } else {
                d.insert(s("background-size"), s("cover"));
                d.insert(s("background-position"), s("center"));
            }
            if self.background_parallax {
                d.insert(s("background-attachment"), s("fixed"));
            }
        }

        match &self.anchor {
            Some(_)
                if !((self.left.is_some() || self.right.is_some())
                    && (self.top.is_some() || self.bottom.is_some())) =>
            {
                let position = if let Some(ref position) = self.position {
                    position
                } else {
                    &ftd::Position::TopLeft
                };
                for (key, value) in non_static_container_align(position, self.inner) {
                    d.insert(s(key.as_str()), value);
                }
            }
            _ => {}
        }

        let translate = get_translate(
            &self.move_left,
            &self.move_right,
            &self.move_up,
            &self.move_down,
            &self.scale,
            &self.scale_x,
            &self.scale_y,
            &self.rotate,
            doc_id,
        )
        .unwrap();

        if let Some(p) = translate {
            let data = if let Some(d) = d.get_mut("transform") {
                format!("{} {}", d, p)
            } else {
                p
            };
            d.insert(s("transform"), data);
        }

        if let Some(p) = &self.border_style {
            d.insert(s("border-style"), s(p));
        } else {
            d.insert(s("border-style"), s("solid"));
        }
        d.insert(s("border-width"), format!("{}px", self.border_width));
        d.insert(s("border-radius"), format!("{}px", self.border_radius));
        d.insert(s("box-sizing"), s("border-box"));

        if let Some(ref p) = self.white_space {
            d.insert(s("white-space"), s(p));
        } else {
            d.insert(s("white-space"), s("initial"));
        }

        d
    }

    fn attrs(&self) -> ftd::Map {
        let mut d: ftd::Map = Default::default();
        if let Some(ref id) = self.data_id {
            d.insert(s("data-id"), escape(id));
        }
        if let Some(ref id) = self.id {
            d.insert(s("id"), escape(id));
        }
        // TODO(move-to-ftd): the link should be escaped
        if let Some(ref link) = self.link {
            d.insert(s("href"), link.to_string());
        }
        if self.open_in_new_tab {
            d.insert(s("target"), escape("_blank"));
        }
        if let Some(ref link) = self.submit {
            if cfg!(feature = "realm") {
                d.insert(
                    s("onclick"),
                    format!("window.REALM_SUBMIT('{}');", escape(link)),
                );
            } else {
                d.insert(s("onclick"), "this.submit()".to_string());
            }
        }
        d
    }
}
impl ftd::Container {
    fn style(&self) -> ftd::Map {
        let mut d: ftd::Map = Default::default();
        let mut count = count_children_with_absolute_parent(&self.children);
        if let Some((_, _, ref ext_children)) = self.external_children {
            count += count_children_with_absolute_parent(ext_children);
        }
        if count != 0 {
            d.insert(s("position"), s("relative"));
        }
        return d;

        fn count_children_with_absolute_parent(children: &[ftd::Element]) -> usize {
            children
                .iter()
                .filter(|v| {
                    let mut bool = false;
                    if let Some(common) = v.get_common() {
                        if Some(ftd::Anchor::Parent) == common.anchor {
                            bool = true;
                        }
                    }
                    bool
                })
                .count()
        }
    }
    fn children_style(&self) -> ftd::Map {
        let d: ftd::Map = Default::default();
        d
    }

    fn attrs(&self) -> ftd::Map {
        let d: ftd::Map = Default::default();
        d
    }
    fn add_class(&self) -> Vec<String> {
        let d: Vec<String> = Default::default();
        d
    }
}

pub fn escape(s: &str) -> String {
    let s = s.replace('>', "\\u003E");
    let s = s.replace('<', "\\u003C");
    s.replace('&', "\\u0026")
}

fn s(s: &str) -> String {
    s.to_string()
}

pub fn color(c: &ftd::ColorValue) -> String {
    let ftd::ColorValue { r, g, b, alpha } = c;
    format!("rgba({},{},{},{})", r, g, b, alpha)
}

pub fn length(l: &ftd::Length, f: &str) -> (String, String) {
    let s = f.to_string();
    match l {
        ftd::Length::Fill => (s, "100%".to_string()),
        ftd::Length::Auto => (s, "auto".to_string()),
        ftd::Length::Px { value } => (s, format!("{}px", value)),
        ftd::Length::Portion { value } => ("flex-grow".to_string(), value.to_string()),
        ftd::Length::Percent { value } => (s, format!("{}%", value)),
        ftd::Length::FitContent => (s, "fit-content".to_string()),
        ftd::Length::Calc { value } => (s, format!("calc({})", value)),
        ftd::Length::VH { value } => (s, format!("{}vh", value)),
        ftd::Length::VW { value } => (s, format!("{}vw", value)),

        _ => (s, "100%".to_string()),
        //        ftd::Length::Shrink => (s, "width".to_string()),   TODO
    }
}

fn text_align(l: &ftd::TextAlign) -> (String, String) {
    match l {
        ftd::TextAlign::Center => ("text-align".to_string(), "center".to_string()),
        ftd::TextAlign::Left => ("text-align".to_string(), "left".to_string()),
        ftd::TextAlign::Right => ("text-align".to_string(), "right".to_string()),
        ftd::TextAlign::Justify => ("text-align".to_string(), "justify".to_string()),
    }
}

pub fn overflow(l: &ftd::Overflow, f: &str) -> (String, String) {
    let s = f.to_string();
    match l {
        ftd::Overflow::Auto => (s, "auto".to_string()),
        ftd::Overflow::Hidden => (s, "hidden".to_string()),
        ftd::Overflow::Scroll => (s, "scroll".to_string()),
        ftd::Overflow::Visible => (s, "visible".to_string()),
    }
}

fn gradient(d: &ftd::GradientDirection, c: &[ftd::ColorValue]) -> String {
    let color = c.iter().map(color).collect::<Vec<String>>().join(",");
    let gradient_style = match d {
        ftd::GradientDirection::BottomToTop => "linear-gradient(to top ".to_string(),
        ftd::GradientDirection::TopToBottom => "linear-gradient(to bottom ".to_string(),
        ftd::GradientDirection::LeftToRight => "linear-gradient(to right".to_string(),
        ftd::GradientDirection::RightToLeft => "linear-gradient(to left".to_string(),
        ftd::GradientDirection::BottomRightToTopLeft => "linear-gradient(to top left".to_string(),
        ftd::GradientDirection::TopLeftBottomRight => "linear-gradient(to bottom right".to_string(),
        ftd::GradientDirection::BottomLeftToTopRight => "linear-gradient(to top right".to_string(),
        ftd::GradientDirection::TopRightToBottomLeft => {
            "linear-gradient(to bottom left".to_string()
        }
        ftd::GradientDirection::Center => "radial-gradient(circle ".to_string(),
        ftd::GradientDirection::Angle { value } => format!("linear-gradient({}deg", value),
    };
    format!("{}, {} )", gradient_style, color)
}

pub fn anchor(l: &ftd::Anchor) -> String {
    match l {
        ftd::Anchor::Parent => ("absolute".to_string()),
        ftd::Anchor::Window => ("fixed".to_string()),
    }
}

fn row_align(l: &ftd::Position) -> Vec<(String, String)> {
    match l {
        ftd::Position::Center => vec![
            ("align-self".to_string(), "center".to_string()),
            ("margin-bottom".to_string(), "auto".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
        ],
        ftd::Position::Top => vec![
            ("align-self".to_string(), "center".to_string()),
            ("margin-bottom".to_string(), "auto".to_string()),
        ],
        ftd::Position::Bottom => vec![
            ("align-self".to_string(), "center".to_string()),
            ("margin-bottom".to_string(), "0".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
        ],
        _ => vec![],
    }
}

pub(crate) fn column_align(l: &ftd::Position) -> Vec<(String, String)> {
    match l {
        ftd::Position::Center => vec![
            ("align-self".to_string(), "center".to_string()),
            ("margin-left".to_string(), "auto".to_string()),
            ("margin-right".to_string(), "auto".to_string()),
        ],
        ftd::Position::Left => vec![
            ("align-self".to_string(), "flex-start".to_string()),
            ("margin-right".to_string(), "auto".to_string()),
        ],
        ftd::Position::Right => vec![
            ("align-self".to_string(), "flex-end".to_string()),
            ("margin-left".to_string(), "auto".to_string()),
        ],
        _ => vec![],
    }
}

fn grid_align(l: &ftd::Position) -> Vec<(String, String)> {
    match l {
        ftd::Position::Center => vec![
            ("align-self".to_string(), "center".to_string()),
            ("justify-self".to_string(), "center".to_string()),
        ],
        ftd::Position::Top => vec![
            ("align-self".to_string(), "start".to_string()),
            ("justify-self".to_string(), "center".to_string()),
        ],
        ftd::Position::Left => vec![
            ("align-self".to_string(), "center".to_string()),
            ("justify-self".to_string(), "start".to_string()),
        ],
        ftd::Position::Right => vec![
            ("align-self".to_string(), "center".to_string()),
            ("justify-self".to_string(), "end".to_string()),
        ],
        ftd::Position::Bottom => vec![
            ("align-self".to_string(), "end".to_string()),
            ("justify-self".to_string(), "center".to_string()),
        ],
        ftd::Position::TopLeft => vec![
            ("align-self".to_string(), "flex-start".to_string()),
            ("justify-self".to_string(), "flex-start".to_string()),
        ],
        ftd::Position::TopRight => vec![
            ("align-self".to_string(), "flex-start".to_string()),
            ("justify-self".to_string(), "flex-end".to_string()),
        ],
        ftd::Position::BottomLeft => vec![
            ("align-self".to_string(), "flex-end".to_string()),
            ("justify-self".to_string(), "flex-start".to_string()),
        ],
        ftd::Position::BottomRight => vec![
            ("align-self".to_string(), "flex-end".to_string()),
            ("justify-self".to_string(), "flex-end".to_string()),
        ],
    }
}

/*fn container_align(l: &ftd::Position) -> Vec<(String, String)> {
    match l {
        ftd::Position::Center => vec![
            ("align-self".to_string(), "center".to_string()),
            ("margin-bottom".to_string(), "auto".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
        ],
        ftd::Position::Top => vec![
            ("align-self".to_string(), "center".to_string()),
            ("margin-bottom".to_string(), "auto".to_string()),
        ],
        ftd::Position::Left => vec![
            ("align-self".to_string(), "flex-start".to_string()),
            ("margin-bottom".to_string(), "auto".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
        ],
        ftd::Position::Right => vec![
            ("align-self".to_string(), "flex-end".to_string()),
            ("margin-bottom".to_string(), "auto".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
            ("margin-left".to_string(), "auto".to_string()),
        ],
        ftd::Position::Bottom => vec![
            ("align-self".to_string(), "center".to_string()),
            ("margin-bottom".to_string(), "0".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
        ],
        ftd::Position::TopLeft => vec![("align-self".to_string(), "flex-start".to_string())],
        ftd::Position::TopRight => vec![
            ("align-self".to_string(), "flex-end".to_string()),
            ("margin-left".to_string(), "auto".to_string()),
        ],
        ftd::Position::BottomLeft => vec![
            ("align-self".to_string(), "flex-start".to_string()),
            ("margin-bottom".to_string(), "0".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
        ],
        ftd::Position::BottomRight => vec![
            ("align-self".to_string(), "flex-end".to_string()),
            ("margin-bottom".to_string(), "0".to_string()),
            ("margin-top".to_string(), "auto".to_string()),
            ("margin-left".to_string(), "auto".to_string()),
        ],
    }
}*/

fn non_static_container_align(l: &ftd::Position, inner: bool) -> Vec<(String, String)> {
    match l {
        ftd::Position::Center => vec![
            ("left".to_string(), "50%".to_string()),
            ("top".to_string(), "50%".to_string()),
            ("transform".to_string(), "translate(-50%,-50%)".to_string()),
        ],
        ftd::Position::Top => {
            if inner {
                vec![
                    ("top".to_string(), "0".to_string()),
                    ("left".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateX(-50%)".to_string()),
                ]
            } else {
                vec![
                    ("bottom".to_string(), "100%".to_string()),
                    ("left".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateX(-50%)".to_string()),
                ]
            }
        }
        ftd::Position::Left => {
            if inner {
                vec![
                    ("left".to_string(), "0".to_string()),
                    ("top".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateY(-50%)".to_string()),
                ]
            } else {
                vec![
                    ("right".to_string(), "100%".to_string()),
                    ("top".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateY(-50%)".to_string()),
                ]
            }
        }
        ftd::Position::Right => {
            if inner {
                vec![
                    ("right".to_string(), "0".to_string()),
                    ("top".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateY(-50%)".to_string()),
                ]
            } else {
                vec![
                    ("left".to_string(), "100%".to_string()),
                    ("top".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateY(-50%)".to_string()),
                ]
            }
        }
        ftd::Position::Bottom => {
            if inner {
                vec![
                    ("bottom".to_string(), "0".to_string()),
                    ("left".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateX(-50%)".to_string()),
                ]
            } else {
                vec![
                    ("top".to_string(), "100%".to_string()),
                    ("left".to_string(), "50%".to_string()),
                    ("transform".to_string(), "translateX(-50%)".to_string()),
                ]
            }
        }
        ftd::Position::TopLeft => {
            if inner {
                vec![
                    ("top".to_string(), "0".to_string()),
                    ("left".to_string(), "0".to_string()),
                ]
            } else {
                vec![
                    ("bottom".to_string(), "100%".to_string()),
                    ("right".to_string(), "100%".to_string()),
                ]
            }
        }
        ftd::Position::TopRight => {
            if inner {
                vec![
                    ("top".to_string(), "0".to_string()),
                    ("right".to_string(), "0".to_string()),
                ]
            } else {
                vec![
                    ("bottom".to_string(), "100%".to_string()),
                    ("left".to_string(), "100%".to_string()),
                ]
            }
        }
        ftd::Position::BottomLeft => {
            if inner {
                vec![
                    ("bottom".to_string(), "0".to_string()),
                    ("left".to_string(), "0".to_string()),
                ]
            } else {
                vec![
                    ("top".to_string(), "100%".to_string()),
                    ("right".to_string(), "100%".to_string()),
                ]
            }
        }
        ftd::Position::BottomRight => {
            if inner {
                vec![
                    ("bottom".to_string(), "0".to_string()),
                    ("right".to_string(), "0".to_string()),
                ]
            } else {
                vec![
                    ("top".to_string(), "100%".to_string()),
                    ("left".to_string(), "100%".to_string()),
                ]
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn get_translate(
    left: &Option<i64>,
    right: &Option<i64>,
    up: &Option<i64>,
    down: &Option<i64>,
    scale: &Option<f64>,
    scale_x: &Option<f64>,
    scale_y: &Option<f64>,
    rotate: &Option<i64>,
    doc_id: &str,
) -> ftd::p1::Result<Option<String>> {
    let mut translate = match (left, right, up, down) {
        (Some(_), Some(_), Some(_), Some(_)) => {
            return ftd::e2(
                "move-up, move-down, move-left and move-right all 4 can't be used at once!",
                doc_id,
                0, // TODO
            );
        }
        (Some(_), Some(_), _, _) => {
            return ftd::e2(
                "move-left, move-right both can't be used at once!",
                doc_id,
                0, // TODO
            );
        }
        (_, _, Some(_), Some(_)) => {
            // TODO
            return ftd::e2("move-up, move-down both can't be used at once!", doc_id, 0);
        }
        (Some(l), None, None, None) => Some(format!("translateX(-{}px) ", l)),
        (Some(l), None, Some(u), None) => Some(format!("translate(-{}px, -{}px) ", l, u)),
        (Some(l), None, None, Some(d)) => Some(format!("translate(-{}px, {}px) ", l, d)),
        (None, Some(r), None, None) => Some(format!("translateX({}px) ", r)),
        (None, Some(r), Some(u), None) => Some(format!("translate({}px, -{}px) ", r, u)),
        (None, Some(r), None, Some(d)) => Some(format!("translate({}px, {}px) ", r, d)),
        (None, None, Some(u), None) => Some(format!("translateY(-{}px) ", u)),
        (None, None, None, Some(d)) => Some(format!("translateY({}px) ", d)),
        _ => None,
    };

    if let Some(ref scale) = scale {
        if let Some(d) = translate {
            translate = Some(format!("{} scale({})", d, scale));
        } else {
            translate = Some(format!("scale({})", scale));
        };
    }
    if let Some(ref scale) = scale_x {
        if let Some(d) = translate {
            translate = Some(format!("{} scaleX({})", d, scale));
        } else {
            translate = Some(format!("scaleX({})", scale));
        };
    }
    if let Some(ref scale) = scale_y {
        if let Some(d) = translate {
            translate = Some(format!("{} scaleY({})", d, scale));
        } else {
            translate = Some(format!("scaleY({})", scale));
        };
    }
    if let Some(ref rotate) = rotate {
        if let Some(d) = translate {
            translate = Some(format!("{} rotate({}deg)", d, rotate));
        } else {
            translate = Some(format!("rotate({}deg)", rotate));
        };
    }
    Ok(translate)
}

pub fn spacing(l: &ftd::Spacing, f: &str) -> (String, String) {
    match l {
        ftd::Spacing::SpaceEvenly => (s("justify-content"), s("space-evenly")),
        ftd::Spacing::SpaceBetween => (s("justify-content"), s("space-between")),
        ftd::Spacing::SpaceAround => (s("justify-content"), s("space-around")),
        ftd::Spacing::Absolute { value } => (s(f), s(value)),
    }
}

#[derive(Debug, PartialEq)]
pub struct ExecuteDoc<'a> {
    pub name: &'a str,
    pub aliases: &'a std::collections::BTreeMap<String, String>,
    pub bag: &'a std::collections::BTreeMap<String, crate::p2::Thing>,
    pub instructions: &'a [ftd::Instruction],
    pub arguments: &'a std::collections::BTreeMap<String, crate::Value>,
    pub invocations: &'a mut std::collections::BTreeMap<
        String,
        Vec<std::collections::BTreeMap<String, crate::Value>>,
    >,
    pub root_name: Option<&'a str>,
}

impl<'a> ExecuteDoc<'a> {
    pub(crate) fn execute(
        &mut self,
        parent_container: &[usize],
        all_locals: &ftd_rt::Map,
    ) -> crate::p1::Result<crate::component::ElementWithContainer> {
        let mut index = 0;
        self.execute_(&mut index, false, parent_container, all_locals, None)
    }

    fn execute_(
        &mut self,
        index: &mut usize,
        is_external: bool,
        parent_container: &[usize],
        all_locals: &ftd_rt::Map,
        parent_id: Option<String>,
    ) -> crate::p1::Result<crate::component::ElementWithContainer> {
        let mut current_container: Vec<usize> = Default::default();
        let mut named_containers: std::collections::BTreeMap<String, Vec<Vec<usize>>> =
            Default::default();
        let mut children: Vec<ftd_rt::Element> = vec![];

        while *index < self.instructions.len() {
            let doc = crate::p2::TDoc {
                name: self.name,
                aliases: self.aliases,
                bag: self.bag,
            };

            let local_container = {
                let mut local_container = parent_container.to_vec();
                local_container.append(&mut current_container.to_vec());
                let current_length = {
                    let mut current = &children;
                    for i in current_container.iter() {
                        current = match &current[*i] {
                            ftd_rt::Element::Row(ref r) => &r.container.children,
                            ftd_rt::Element::Column(ref r) => &r.container.children,
                            _ => unreachable!(),
                        };
                    }
                    current.len()
                };
                local_container.push(current_length);
                local_container
            };

            match &self.instructions[*index] {
                ftd::Instruction::ChangeContainer { name: c } => {
                    if !named_containers.contains_key(c)
                        && is_external
                        && !match_parent_id(c, &parent_id)
                    {
                        *index -= 1;
                        return Ok(crate::component::ElementWithContainer {
                            element: ftd_rt::Element::Null,
                            children,
                            child_container: Some(named_containers),
                        });
                    }
                    change_container(c, &mut current_container, &mut named_containers, &parent_id)?;
                }
                ftd::Instruction::Component {
                    parent,
                    children: inner,
                } => {
                    assert!(self.arguments.is_empty()); // This clause cant have arguments
                    let crate::component::ElementWithContainer {
                        element,
                        child_container,
                        ..
                    } = parent.super_call(
                        inner,
                        &doc,
                        self.arguments,
                        self.invocations,
                        all_locals,
                        &local_container,
                    )?;

                    children = self.add_element(
                        children,
                        &mut current_container,
                        &mut named_containers,
                        element,
                        child_container,
                        index,
                        parent_container,
                        &Default::default(),
                    )?;
                }
                ftd::Instruction::ChildComponent { child: f } => {
                    let crate::component::ElementWithContainer {
                        element: e,
                        child_container,
                        ..
                    } = f.call(
                        &doc,
                        self.arguments,
                        self.invocations,
                        true,
                        self.root_name,
                        all_locals,
                        &local_container,
                    )?;

                    children = self.add_element(
                        children,
                        &mut current_container,
                        &mut named_containers,
                        e,
                        child_container,
                        index,
                        parent_container,
                        all_locals,
                    )?;
                }
                ftd::Instruction::RecursiveChildComponent { child: f } => {
                    let elements = f.recursive_call(
                        &doc,
                        self.arguments,
                        self.invocations,
                        true,
                        self.root_name,
                        all_locals,
                        &local_container,
                    )?;
                    for e in elements {
                        children = self.add_element(
                            children,
                            &mut current_container,
                            &mut named_containers,
                            e.element,
                            None,
                            index,
                            parent_container,
                            all_locals,
                        )?
                    }
                }
            }
            *index += 1;
        }

        Ok(crate::component::ElementWithContainer {
            element: ftd_rt::Element::Null,
            children,
            child_container: Some(named_containers),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn add_element(
        &mut self,
        mut main: Vec<ftd_rt::Element>,
        current_container: &mut Vec<usize>,
        named_containers: &mut std::collections::BTreeMap<String, Vec<Vec<usize>>>,
        e: ftd_rt::Element,
        container: Option<std::collections::BTreeMap<String, Vec<Vec<usize>>>>,
        index: &mut usize,
        parent_container: &[usize],
        all_locals: &ftd_rt::Map,
    ) -> crate::p1::Result<Vec<ftd_rt::Element>> {
        let mut current = &mut main;
        for i in current_container.iter() {
            current = match &mut current[*i] {
                ftd_rt::Element::Row(ref mut r) => &mut r.container.children,
                ftd_rt::Element::Column(ref mut r) => &mut r.container.children,
                _ => unreachable!(),
            };
        }
        let len = current.len();
        let mut container_id = None;
        let parent_id = e.container_id();
        if let Some(ref v) = parent_id {
            let mut c = current_container.clone();
            c.push(len);
            container_id = Some(v.clone());
            if let Some(val) = named_containers.get_mut(v.as_str()) {
                val.push(c);
            } else {
                named_containers.insert(v.to_string(), vec![c]);
            }
        }

        if let Some(child_container) = container {
            let mut c = current_container.clone();
            c.push(len);
            update_named_container(&c, named_containers, &child_container, container_id, true);
        }

        let open_id = e.is_open_container().1;
        let container_id = e.container_id();
        let is_open = e.is_open_container().0;

        current.push(e);

        if is_open {
            current_container.push(len);
            let mut new_parent_container = parent_container.to_vec();
            new_parent_container.append(&mut current_container.to_vec());

            let container = match current.last_mut() {
                Some(ftd_rt::Element::Column(ftd_rt::Column {
                    ref mut container, ..
                }))
                | Some(ftd_rt::Element::Row(ftd_rt::Row {
                    ref mut container, ..
                })) => {
                    *index += 1;
                    let child = self.execute_(
                        index,
                        true,
                        &new_parent_container,
                        all_locals,
                        parent_id.clone(),
                    )?;
                    container.children.extend(child.children);
                    child.child_container
                }
                _ => unreachable!(),
            };

            if let Some(child_container) = container {
                update_named_container(
                    current_container,
                    named_containers,
                    &child_container,
                    None,
                    false,
                );
            }
        }

        if let Some(id) = open_id {
            let open_id = container_id.map_or(id.clone(), |v| format!("{}#{}", v, id));

            let container =
                get_external_children(current, &open_id, named_containers, current_container);

            let id = if id.contains('.') {
                ftd::p2::utils::split(id, ".")?.1
            } else {
                id
            };

            current_container.push(len);
            let mut new_parent_container = parent_container.to_vec();
            new_parent_container.append(&mut current_container.to_vec());

            match current.last_mut() {
                Some(ftd_rt::Element::Column(ftd_rt::Column {
                    container: ref mut c,
                    ..
                }))
                | Some(ftd_rt::Element::Row(ftd_rt::Row {
                    container: ref mut c,
                    ..
                })) => {
                    *index += 1;
                    let child = self.execute_(
                        index,
                        true,
                        &new_parent_container,
                        &Default::default(),
                        parent_id,
                    )?;
                    let external_children = {
                        if child.children.is_empty() {
                            vec![]
                        } else {
                            let mut main = ftd::p2::interpreter::default_column();
                            main.container.children = child.children;
                            vec![ftd_rt::Element::Column(main)]
                        }
                    };
                    c.external_children = Some((id, container, external_children));
                }
                _ => unreachable!(),
            }
        }
        Ok(main)
    }
}

fn get_external_children(
    children: &[ftd_rt::Element],
    open_id: &str,
    named_containers: &std::collections::BTreeMap<String, Vec<Vec<usize>>>,
    current_container: &[usize],
) -> Vec<Vec<usize>> {
    let open_id = if !open_id.contains('#') {
        format!("#{}", open_id.replace(".", "#"))
    } else {
        open_id.to_string()
    };

    let container = match named_containers.get(&open_id) {
        Some(c) => {
            let mut container = vec![];
            for c in c {
                let matching = c
                    .iter()
                    .zip(current_container.iter())
                    .filter(|&(a, b)| a == b)
                    .count();
                if let Some(cc) = c.get(matching) {
                    if matching == current_container.len() && cc == &(children.len() - 1) {
                        container.push(c[matching + 1..].to_vec());
                    }
                }
            }
            container
        }
        None => vec![],
    };
    container
}

fn match_parent_id(c: &str, parent_id: &Option<String>) -> bool {
    if let Some(p) = parent_id {
        if c == p {
            return true;
        }
    }
    false
}

fn change_container(
    name: &str,
    current_container: &mut Vec<usize>,
    named_containers: &mut std::collections::BTreeMap<String, Vec<Vec<usize>>>,
    parent_id: &Option<String>,
) -> crate::p1::Result<()> {
    if name == "ftd#main" || match_parent_id(name, parent_id) {
        *current_container = vec![];
        return Ok(());
    }
    *current_container = match named_containers.get(name) {
        Some(v) => v.get(0).unwrap().to_owned(),
        None => {
            return crate::e2("no such container", name);
        }
    };
    Ok(())
}

fn update_named_container(
    current_container: &[usize],
    named_containers: &mut std::collections::BTreeMap<String, Vec<Vec<usize>>>,
    child_container: &std::collections::BTreeMap<String, Vec<Vec<usize>>>,
    container_id: Option<String>,
    key_with_container: bool,
) {
    for (key, value) in child_container.iter() {
        for value in value.iter() {
            let mut hierarchy = current_container.to_vec();
            let mut p2 = value.clone();
            hierarchy.append(&mut p2);
            let container_id = if key_with_container {
                container_id
                    .clone()
                    .map_or(format!("#{}", key), |v| format!("{}#{}", v, key))
            } else {
                key.clone()
            };
            if let Some(val) = named_containers.get_mut(container_id.as_str()) {
                val.push(hierarchy);
            } else {
                named_containers.insert(container_id, vec![hierarchy]);
            }
        }
    }
}
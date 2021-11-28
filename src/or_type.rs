#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrType {
    pub name: String,
    pub variants: Vec<crate::p2::Record>,
}

impl OrType {
    pub fn from_p1(p1: &crate::p1::Section, doc: &crate::p2::TDoc) -> crate::p1::Result<Self> {
        let or_type_name = ftd::get_name("or-type", p1.name.as_str(), doc.name)?;
        let name = doc.format_name(or_type_name);
        let mut variants: Vec<crate::p2::Record> = Default::default();
        for s in p1.sub_sections.0.iter() {
            if s.is_commented {
                continue;
            }
            variants.push(crate::p2::Record::from_p1(
                format!("record {}.{}", or_type_name, s.name.as_str()).as_str(),
                &s.header,
                doc,
                p1.line_number,
            )?);
        }
        Ok(OrType { name, variants })
    }

    pub fn create(
        &self,
        p1: &crate::p1::Section,
        variant: String,
        doc: &crate::p2::TDoc,
    ) -> crate::p1::Result<crate::Value> {
        for v in self.variants.iter() {
            if v.name
                == doc.resolve_name(
                    p1.line_number,
                    format!("{}.{}", self.name, variant.as_str()).as_str(),
                )?
            {
                return Ok(crate::Value::OrType {
                    variant,
                    name: self.name.to_string(),
                    fields: v.fields(p1, doc)?,
                });
            }
        }

        ftd::e2(
            format!("{} is not a valid variant for {}", variant, self.name),
            doc.name,
            p1.line_number,
        )
    }
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn basic() {
        let mut bag = default_bag();

        bag.insert(s("foo/bar#entity"), entity());
        bag.insert(
            s("foo/bar#abrar"),
            crate::p2::Thing::Variable(crate::Variable {
                name: s("abrar"),
                value: crate::Value::OrType {
                    name: s("foo/bar#entity"),
                    variant: s("person"),
                    fields: abrar(),
                },
                conditions: vec![],
            }),
        );
        bag.insert(
            "foo/bar#x".to_string(),
            crate::p2::Thing::Variable(crate::Variable {
                name: "x".to_string(),
                value: crate::Value::Integer { value: 10 },
                conditions: vec![],
            }),
        );

        p!(
            "
            -- $x: 10

            -- or-type entity:

            --- person:
            caption name:
            string address:
            body bio:
            integer age:

            --- company:
            caption name:
            string industry:

            -- entity.person $abrar: Abrar Khan2
            age: $x
            address: Bihar2

            Software developer working at fifthtry2.
            ",
            (bag, default_column()),
        );
    }
}
//! Structured data as one tree (FR-AEO-06), read from JSON-LD, microdata and RDFa, so the same
//! markup in any of the three formats gives the same nodes.

use scraper::{ElementRef, Html};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rules::kit::{all, collapse, text};

/// LocalBusiness and the subtypes sites use most.
pub(crate) const LOCAL_TYPES: [&str; 11] = [
    "LocalBusiness",
    "Restaurant",
    "Store",
    "FoodEstablishment",
    "LodgingBusiness",
    "ProfessionalService",
    "MedicalBusiness",
    "Dentist",
    "LegalService",
    "AutoRepair",
    "HomeAndConstructionBusiness",
];

/// Which markup a top-level item came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SchemaSource {
    JsonLd,
    Microdata,
    Rdfa,
}

/// One structured data item found on the page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SchemaItem {
    pub source: SchemaSource,
    pub node: SchemaNode,
}

/// A typed thing, such as an Article, with its properties sorted by name.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SchemaNode {
    /// Short names: `Article`, not `https://schema.org/Article`. Empty when untyped.
    pub types: Vec<String>,
    pub properties: Vec<SchemaProperty>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SchemaProperty {
    pub name: String,
    pub value: SchemaValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SchemaValue {
    Text(String),
    Node(SchemaNode),
}

impl SchemaNode {
    pub fn is(&self, ty: &str) -> bool {
        self.types.iter().any(|t| t == ty)
    }

    /// Every value of a property.
    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a SchemaValue> + 'a {
        self.properties.iter().filter(move |p| p.name == name).map(|p| &p.value)
    }

    pub fn has(&self, name: &str) -> bool {
        self.values(name).any(|v| match v {
            SchemaValue::Text(t) => !t.trim().is_empty(),
            SchemaValue::Node(_) => true,
        })
    }

    /// Child nodes of a property.
    pub fn nodes<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a SchemaNode> + 'a {
        self.values(name).filter_map(|v| match v {
            SchemaValue::Node(n) => Some(n),
            SchemaValue::Text(_) => None,
        })
    }

    /// This node and every node under it, depth first.
    pub fn walk(&self) -> Vec<&SchemaNode> {
        let mut out = vec![self];
        for p in &self.properties {
            if let SchemaValue::Node(n) = &p.value {
                out.extend(n.walk());
            }
        }
        out
    }

    fn sorted(mut self) -> Self {
        // Stable: values of one property keep document order.
        self.properties.sort_by(|a, b| a.name.cmp(&b.name));
        self
    }
}

/// `https://schema.org/Article`, `schema:Article` and `Article` all give `Article`.
fn short_type(t: &str) -> String {
    let t = t.trim().trim_end_matches('/');
    t.rsplit(['/', '#', ':']).next().unwrap_or(t).to_string()
}

/// Every item on the page: JSON-LD first, then microdata, then RDFa.
pub(crate) fn extract(dom: &Html) -> Vec<SchemaItem> {
    let mut out = Vec::new();
    for el in all(dom, crate::rules::JSON_LD) {
        if let Ok(v) = serde_json::from_str::<Value>(&text(el)) {
            out.extend(jsonld_items(&v).into_iter().map(|node| SchemaItem { source: SchemaSource::JsonLd, node }));
        }
    }
    for el in all(dom, "[itemscope]") {
        // Top level: not the value of another item's property.
        if el.value().attr("itemprop").is_none() {
            out.push(SchemaItem { source: SchemaSource::Microdata, node: microdata_node(el) });
        }
    }
    for el in all(dom, "[typeof]") {
        if el.value().attr("property").is_none() {
            out.push(SchemaItem { source: SchemaSource::Rdfa, node: rdfa_node(el) });
        }
    }
    out
}

fn jsonld_items(v: &Value) -> Vec<SchemaNode> {
    match v {
        Value::Array(a) => a.iter().flat_map(jsonld_items).collect(),
        Value::Object(o) => match o.get("@graph") {
            Some(g) => jsonld_items(g),
            None => vec![jsonld_node(o)],
        },
        _ => Vec::new(),
    }
}

fn jsonld_node(o: &serde_json::Map<String, Value>) -> SchemaNode {
    let mut node = SchemaNode::default();
    for (key, value) in o {
        match key.as_str() {
            "@type" => match value {
                Value::String(s) => node.types.push(short_type(s)),
                Value::Array(a) => node.types.extend(a.iter().filter_map(Value::as_str).map(short_type)),
                _ => {}
            },
            k if k.starts_with('@') => {}
            k => push_json(&mut node, &short_type(k), value),
        }
    }
    node.sorted()
}

fn push_json(node: &mut SchemaNode, name: &str, value: &Value) {
    let value = match value {
        Value::Array(a) => return a.iter().for_each(|v| push_json(node, name, v)),
        Value::Object(o) => SchemaValue::Node(jsonld_node(o)),
        Value::String(s) => SchemaValue::Text(s.clone()),
        Value::Number(n) => SchemaValue::Text(n.to_string()),
        Value::Bool(b) => SchemaValue::Text(b.to_string()),
        Value::Null => return,
    };
    node.properties.push(SchemaProperty { name: name.to_string(), value });
}

/// The value of a microdata or RDFa property element, per the HTML microdata rules.
fn element_value(el: ElementRef, content_attr: bool) -> String {
    let e = el.value();
    if content_attr && let Some(c) = e.attr("content") {
        return c.to_string();
    }
    let attr = match e.name() {
        "meta" => "content",
        "a" | "area" | "link" => "href",
        "img" | "audio" | "video" | "source" | "iframe" | "embed" | "track" => "src",
        "object" => "data",
        "time" => "datetime",
        "data" | "meter" => "value",
        _ => "",
    };
    e.attr(attr).map(str::to_string).unwrap_or_else(|| collapse(&text(el)))
}

/// Descendants of `root` that belong to it: not inside a nested item.
fn own_descendants<'a>(root: ElementRef<'a>, scope: &str) -> Vec<ElementRef<'a>> {
    let mut out = Vec::new();
    let mut stack: Vec<ElementRef> = root.children().filter_map(ElementRef::wrap).collect();
    stack.reverse();
    while let Some(el) = stack.pop() {
        out.push(el);
        if el.value().attr(scope).is_none() {
            let mut kids: Vec<ElementRef> = el.children().filter_map(ElementRef::wrap).collect();
            kids.reverse();
            stack.extend(kids);
        }
    }
    out
}

fn microdata_node(item: ElementRef) -> SchemaNode {
    let types = item.value().attr("itemtype").unwrap_or("").split_whitespace().map(short_type).collect();
    let mut node = SchemaNode { types, properties: Vec::new() };
    for el in own_descendants(item, "itemscope") {
        let Some(names) = el.value().attr("itemprop") else { continue };
        let value = if el.value().attr("itemscope").is_some() {
            SchemaValue::Node(microdata_node(el))
        } else {
            SchemaValue::Text(element_value(el, el.value().name() == "meta"))
        };
        for name in names.split_whitespace() {
            node.properties.push(SchemaProperty { name: short_type(name), value: value.clone() });
        }
    }
    node.sorted()
}

fn rdfa_node(item: ElementRef) -> SchemaNode {
    let types = item.value().attr("typeof").unwrap_or("").split_whitespace().map(short_type).collect();
    let mut node = SchemaNode { types, properties: Vec::new() };
    for el in own_descendants(item, "typeof") {
        let Some(names) = el.value().attr("property") else { continue };
        let value = if el.value().attr("typeof").is_some() {
            SchemaValue::Node(rdfa_node(el))
        } else {
            SchemaValue::Text(element_value(el, true))
        };
        for name in names.split_whitespace() {
            node.properties.push(SchemaProperty { name: short_type(name), value: value.clone() });
        }
    }
    node.sorted()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(html: &str) -> Vec<(SchemaSource, SchemaNode)> {
        extract(&Html::parse_document(html)).into_iter().map(|i| (i.source, i.node)).collect()
    }

    const JSONLD: &str = r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"Article",
        "headline":"Soil guide","author":{"@type":"Person","name":"Ana"},"datePublished":"2026-01-02",
        "image":["https://example.com/a.jpg"]}</script>"#;
    const MICRODATA: &str = r#"<article itemscope itemtype="https://schema.org/Article">
        <h1 itemprop="headline">Soil guide</h1>
        <span itemprop="author" itemscope itemtype="https://schema.org/Person"><span itemprop="name">Ana</span></span>
        <time itemprop="datePublished" datetime="2026-01-02">2 January</time>
        <img itemprop="image" src="https://example.com/a.jpg"></article>"#;
    const RDFA: &str = r#"<article vocab="https://schema.org/" typeof="Article">
        <h1 property="headline">Soil guide</h1>
        <span property="author" typeof="Person"><span property="name">Ana</span></span>
        <time property="datePublished" datetime="2026-01-02">2 January</time>
        <img property="image" src="https://example.com/a.jpg"></article>"#;

    #[test]
    fn the_three_formats_give_the_same_tree() {
        let (src, json) = nodes(JSONLD).remove(0);
        assert_eq!(src, SchemaSource::JsonLd);
        assert!(json.is("Article"));
        assert_eq!(json.nodes("author").next().map(|a| a.types.clone()), Some(vec!["Person".to_string()]));
        let (src, micro) = nodes(MICRODATA).remove(0);
        assert_eq!(src, SchemaSource::Microdata);
        assert_eq!(micro, json);
        let (src, rdfa) = nodes(RDFA).remove(0);
        assert_eq!(src, SchemaSource::Rdfa);
        assert_eq!(rdfa, json);
    }

    #[test]
    fn graphs_and_type_lists() {
        let n = nodes(
            r#"<script type="application/ld+json">{"@graph":[{"@type":["Organization","Brand"],"name":"A"},
            {"@type":"WebSite","url":"https://a.com","potentialAction":null}]}</script>"#,
        );
        assert_eq!(n.len(), 2);
        assert_eq!(n[0].1.types, ["Organization", "Brand"]);
        assert!(n[1].1.has("url") && !n[1].1.has("potentialAction"));
        assert!(nodes(r#"<script type="application/ld+json">{oops</script>"#).is_empty());
    }
}

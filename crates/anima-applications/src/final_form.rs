//! The final form of a project (contract K10 Section 6.4.4, step 5): the one file that
//! Compose reads. The program writes it itself, as JSON, from the file that passed the check
//! of `compose-rules`: each value with its type, each image by the digest of its pull, the
//! name of the project, each path of the machine as the real path, and what the program adds
//! to or fixes in each service.

mod declared;

use compose_rules::{Checked, Node, Rule, forms::truth};

use crate::{
    Pulled, Refusal,
    forms::is_digest,
    json::{self, Json},
    settings::OwnLimits,
};

/// What the final form needs beside the file.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// The name of the project: `anima-` and the 32 hexadecimal digits of the application.
    pub project: &'a str,
    /// The real path of `binds/` of the application.
    pub binds: &'a str,
    /// The limits of a service at this machine.
    pub own: &'a OwnLimits,
    /// "Open to the network".
    pub open_to_network: bool,
    /// The digest by which the program pulled the image of each service.
    pub digests: &'a [Pulled],
}

/// The final form of one project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalForm {
    /// The text of the file.
    pub text: String,
    /// The name of the folder of each bind mount, each one time.
    pub binds: Vec<String>,
    /// True when the project has a volume or a bind mount: data that `application_back` does
    /// not bring back.
    pub holds_volumes: bool,
}

const fn refused(rule: Rule, service: Option<usize>) -> Refusal {
    Refusal::FileNotPermitted { rule, service }
}

/// True when a text of `node`, a key or a value, holds a NUL.
fn holds_nul(node: &Node) -> bool {
    match node {
        Node::Null => false,
        Node::Text(text) => text.contains('\0'),
        Node::List(items) => items.iter().any(holds_nul),
        Node::Map(entries) => entries
            .iter()
            .any(|(key, value)| key.contains('\0') || holds_nul(value)),
    }
}

/// The image of a service by the digest of its pull: the name with no tag, then `@` and the
/// digest. Thus the engine never starts an image that was on the machine for a different
/// reason (SF-041).
pub(crate) fn image_by_digest(image: &str, digest: &str) -> String {
    let (folders, last) = image
        .rsplit_once('/')
        .map_or(("", image), |(folders, last)| (folders, last));
    let name = last.split(['@', ':']).next().unwrap_or(last);
    let separator = if folders.is_empty() { "" } else { "/" };
    format!("{folders}{separator}{name}@{digest}")
}

/// The digest that `frame` has for the service `name` with the image `image`.
fn digest_of<'a>(frame: &Frame<'a>, name: &str, image: &str) -> Option<&'a str> {
    frame
        .digests
        .iter()
        .find(|pulled| pulled.service == name && pulled.image == image)
        .map(|pulled| pulled.digest.as_str())
        .filter(|digest| is_digest(digest))
        // An image that the file names by a digest is started by that digest and no other.
        .filter(|digest| {
            image
                .split_once('@')
                .is_none_or(|(_, named)| named == *digest)
        })
}

fn services(document: &Node, frame: &Frame<'_>) -> Result<Json, Refusal> {
    let mut members = Vec::new();
    let entries = document
        .get("services")
        .and_then(Node::entries)
        .unwrap_or(&[]);
    for (position, (name, node)) in entries.iter().enumerate() {
        let image = node.get("image").and_then(Node::text);
        let image = image.ok_or(refused(Rule::ImageNotPermitted, Some(position)))?;
        let digest = digest_of(frame, name, image).ok_or(Refusal::NoDigest)?;
        let formed = declared::formed(node, image_by_digest(image, digest), frame)
            .map_err(|_| refused(Rule::ValueNotPermitted, Some(position)))?;
        members.push((name.clone(), formed));
    }
    Ok(Json::Record(members))
}

/// A top-level network: the driver `bridge` when the file names it, and `internal`.
fn network(node: &Node) -> Result<Json, Refusal> {
    let mut members = Vec::new();
    if node.get("driver").and_then(Node::text).is_some() {
        members.push(("driver".to_owned(), Json::text("bridge")));
    }
    if let Some(internal) = node.get("internal").and_then(Node::text) {
        let internal = truth(internal).ok_or(refused(Rule::ValueNotPermitted, None))?;
        members.push(("internal".to_owned(), Json::Truth(internal)));
    }
    Ok(Json::Record(members))
}

/// One top-level key whose entries have names: each entry formed by `reader`.
fn named(
    document: &Node,
    key: &str,
    reader: impl Fn(&Node) -> Result<Json, Refusal>,
) -> Result<Option<(String, Json)>, Refusal> {
    let Some(entries) = document.get(key).and_then(Node::entries) else {
        return Ok(None);
    };
    let mut members = Vec::new();
    for (name, node) in entries {
        members.push((name.clone(), reader(node)?));
    }
    Ok(Some((key.to_owned(), Json::Record(members))))
}

/// The names of the folders of the bind mounts of the file, each one time.
fn binds(document: &Node) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let services = document
        .get("services")
        .and_then(Node::entries)
        .unwrap_or(&[]);
    for (_, node) in services {
        let Some(Node::List(mounts)) = node.get("volumes") else {
            continue;
        };
        for mount in mounts {
            let source = mount.get("source").and_then(Node::text);
            let name = source.and_then(compose_rules::forms::bind_name);
            let is_bind = mount.get("type").and_then(Node::text) == Some("bind");
            if let Some(name) =
                name.filter(|name| is_bind && !names.iter().any(|known| known == name))
            {
                names.push(name.to_owned());
            }
        }
    }
    names
}

/// Makes the final form of `checked` for the project of `frame`.
///
/// # Errors
/// `no_digest` for a service whose image has no digest in `frame`; `file_not_permitted`
/// with `value_not_permitted` for a NUL in a text, and for a value that does not have the
/// form of its rule.
pub fn formed(checked: &Checked, frame: &Frame<'_>) -> Result<FinalForm, Refusal> {
    let document = &checked.document;
    let entries = document
        .get("services")
        .and_then(Node::entries)
        .unwrap_or(&[]);
    if let Some(position) = entries
        .iter()
        .position(|(name, node)| name.contains('\0') || holds_nul(node))
    {
        return Err(refused(Rule::ValueNotPermitted, Some(position)));
    }
    if holds_nul(document) {
        return Err(refused(Rule::ValueNotPermitted, None));
    }
    let mut members = vec![
        ("name".to_owned(), Json::text(frame.project)),
        ("services".to_owned(), services(document, frame)?),
    ];
    let content = |node: &Node| {
        let content = node.get("content").and_then(Node::text);
        let content = content.ok_or(refused(Rule::ValueNotPermitted, None))?;
        Ok(Json::Record(vec![(
            "content".to_owned(),
            Json::text(content),
        )]))
    };
    members.extend(named(document, "volumes", |_| Ok(Json::empty()))?);
    members.extend(named(document, "networks", network)?);
    members.extend(named(document, "configs", content)?);
    let binds = binds(document);
    let volumes = document
        .get("volumes")
        .and_then(Node::entries)
        .unwrap_or(&[]);
    Ok(FinalForm {
        text: json::written(&Json::Record(members)),
        holds_volumes: !binds.is_empty() || !volumes.is_empty(),
        binds,
    })
}

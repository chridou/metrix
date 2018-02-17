use instruments::Descriptive;
use snapshot::{ItemKind, Snapshot};

const TITLE_FIELD_LABEL: &'static str = "title";
const DESCRIPTION_FIELD_LABEL: &'static str = "description";

pub fn put_default_descriptives<T>(what: &T, into: &mut Snapshot, add_descriptive_parts: bool)
where
    T: Descriptive,
{
    if add_descriptive_parts {
        put_descriptives(what, TITLE_FIELD_LABEL, DESCRIPTION_FIELD_LABEL, into);
    }
}

pub fn put_prefixed_descriptives<T>(
    what: &T,
    field_label_prefix: &str,
    into: &mut Snapshot,
    add_descriptive_parts: bool,
) where
    T: Descriptive,
{
    if !add_descriptive_parts {
        return;
    }

    if let Some(title) = what.title() {
        let label = format!("{}_{}", field_label_prefix, TITLE_FIELD_LABEL);
        let title_not_already_there = into.items.iter().find(|&&(ref n, _)| n == &label).is_none();
        if title_not_already_there {
            into.items.push((label, ItemKind::Text(title.to_string())));
        }
    }

    if let Some(description) = what.description() {
        let label = format!("{}_{}", field_label_prefix, DESCRIPTION_FIELD_LABEL);
        let description_not_already_there =
            into.items.iter().find(|&&(ref n, _)| n == &label).is_none();
        if description_not_already_there {
            into.items
                .push((label, ItemKind::Text(description.to_string())));
        }
    }
}

pub fn put_descriptives<T>(
    what: &T,
    title_field_label: &str,
    description_field_label: &str,
    into: &mut Snapshot,
) where
    T: Descriptive,
{
    if let Some(title) = what.title() {
        let title_not_already_there = into.items
            .iter()
            .find(|&&(ref n, _)| n == title_field_label)
            .is_none();
        if title_not_already_there {
            into.items.push((
                title_field_label.to_string(),
                ItemKind::Text(title.to_string()),
            ));
        }
    }

    if let Some(description) = what.description() {
        let description_not_already_there = into.items
            .iter()
            .find(|&&(ref n, _)| n == description_field_label)
            .is_none();
        if description_not_already_there {
            into.items.push((
                description_field_label.to_string(),
                ItemKind::Text(description.to_string()),
            ));
        }
    }
}

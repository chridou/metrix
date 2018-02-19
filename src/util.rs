use Descriptive;
use snapshot::{ItemKind, Snapshot};

const TITLE_FIELD_LABEL: &'static str = "_title";
const DESCRIPTION_FIELD_LABEL: &'static str = "_description";

pub fn put_default_descriptives<T>(what: &T, into: &mut Snapshot, add_descriptive_parts: bool)
where
    T: Descriptive,
{
    if add_descriptive_parts {
        put_descriptives(what, TITLE_FIELD_LABEL, DESCRIPTION_FIELD_LABEL, into);
    }
}

pub fn put_postfixed_descriptives<T>(
    what: &T,
    field_label_postfix: &str,
    into: &mut Snapshot,
    add_descriptive_parts: bool,
) where
    T: Descriptive,
{
    if !add_descriptive_parts {
        return;
    }

    if let Some(title) = what.title() {
        let label = format!("{}_{}", TITLE_FIELD_LABEL, field_label_postfix);
        let title_not_already_there = into.items.iter().find(|&&(ref n, _)| n == &label).is_none();
        if title_not_already_there {
            into.items.push((label, ItemKind::Text(title.to_string())));
        }
    }

    if let Some(description) = what.description() {
        let label = format!("{}_{}", DESCRIPTION_FIELD_LABEL, field_label_postfix);
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

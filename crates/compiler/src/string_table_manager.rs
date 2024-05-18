//! Adapted from <https://github.com/YarnSpinnerTool/YarnSpinner/blob/da39c7195107d8211f21c263e4084f773b84eaff/YarnSpinner.Compiler/StringTableManager.cs>

use crate::output::StringInfo;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use yarnspinner_core::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct StringTableManager(pub HashMap<LineId, StringInfo>);

impl StringTableManager {
    pub(crate) fn contains_implicit_string_tags(&self) -> bool {
        self.values().any(|x| x.is_implicit_tag)
    }

    /// Inserts a new string into the string table, optionally generating a new line ID.
    /// The `is_implicit_tag` field of the `string_info` is automatically set; its original value is ignored.
    ///
    /// ## Returns
    ///
    /// The line ID used for insertion. This will be `line_id` if it is `Some`, otherwise it will be an autogenerated line ID.
    pub(crate) fn insert(
        &mut self,
        line_id: impl Into<Option<LineId>>,
        string_info: StringInfo,
    ) -> LineId {
        let line_id = line_id.into();
        let (line_id, string_info) = if let Some(line_id) = line_id {
            let string_info = StringInfo {
                is_implicit_tag: false,
                ..string_info
            };
            (line_id, string_info)
        } else {
            let line_id = format!(
                "line:{}-{}-{}",
                string_info.file_name,
                string_info.node_name,
                self.len()
            )
            .into();
            let string_info = StringInfo {
                is_implicit_tag: true,
                ..string_info
            };
            (line_id, string_info)
        };
        self.0.insert(line_id.clone(), string_info);
        line_id
    }

    pub(crate) fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

impl Deref for StringTableManager {
    type Target = HashMap<LineId, StringInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StringTableManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<LineId, StringInfo>> for StringTableManager {
    fn from(map: HashMap<LineId, StringInfo>) -> Self {
        Self(map)
    }
}

impl From<StringTableManager> for HashMap<LineId, StringInfo> {
    fn from(manager: StringTableManager) -> Self {
        manager.0
    }
}

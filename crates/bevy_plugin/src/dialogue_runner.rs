use crate::prelude::*;
use crate::project::*;
use bevy::prelude::*;
use std::fmt::Debug;

pub(crate) fn dialogue_plugin(app: &mut App) {
    app.add_system(set_dialogue_programs);
}

#[derive(Debug, Default, Component)]
pub struct DialogueRunner {
    pub(crate) dialogue: Option<Dialogue>,
    pub(crate) variable_storage_override: Option<Box<dyn VariableStorage>>,
    pub(crate) text_provider_override: Option<Box<dyn TextProvider>>,
    pub(crate) line_asset_provider_override: Option<Option<Box<dyn LineAssetProvider>>>,
}

impl DialogueRunner {
    pub fn new() -> Self {
        Self {
            dialogue: None,
            variable_storage_override: None,
            text_provider_override: None,
            line_asset_provider_override: None,
        }
    }

    pub fn override_variable_storage(mut self, storage: Box<dyn VariableStorage>) -> Self {
        self.variable_storage_override = Some(storage);
        self
    }

    pub fn override_text_provider(mut self, provider: Box<dyn TextProvider>) -> Self {
        self.text_provider_override = Some(provider);
        self
    }

    pub fn override_asset_provider(
        mut self,
        provider: impl Into<Option<Box<dyn LineAssetProvider>>>,
    ) -> Self {
        self.line_asset_provider_override = Some(provider.into());
        self
    }
}

fn set_dialogue_programs(
    mut dialogue_runners: Query<&mut DialogueRunner>,
    global_variable_storage: Res<GlobalVariableStorage>,
    global_text_provider: Res<GlobalTextProvider>,
    global_line_asset_provider: Option<Res<GlobalLineAssetProvider>>,
    yarn_compilation: Res<YarnCompilation>,
) {
    let compilation_changed = yarn_compilation.is_changed();
    let dialogue_runners = dialogue_runners
        .iter_mut()
        .filter(|runner| compilation_changed || runner.dialogue.is_none());
    for mut dialogue_runner in dialogue_runners {
        let dialogue = if let Some(dialogue) = &mut dialogue_runner.dialogue {
            dialogue
        } else {
            let text_provider = dialogue_runner
                .text_provider_override
                .as_ref()
                .map(|provider| provider.clone_shallow())
                .unwrap_or_else(|| global_text_provider.0.clone_shallow());
            let variable_storage = dialogue_runner
                .variable_storage_override
                .as_ref()
                .map(|storage| storage.clone_shallow())
                .unwrap_or_else(|| global_variable_storage.0.clone_shallow());
            if dialogue_runner.line_asset_provider_override.is_none() {
                let line_asset_provider = global_line_asset_provider
                    .as_ref()
                    .map(|provider| provider.0.clone_shallow());
                dialogue_runner.line_asset_provider_override = Some(line_asset_provider);
            }
            dialogue_runner.dialogue = Some(Dialogue::new(variable_storage, text_provider));
            dialogue_runner.dialogue.as_mut().unwrap()
        };
        dialogue.replace_program(yarn_compilation.0.program.clone().unwrap());
    }
}
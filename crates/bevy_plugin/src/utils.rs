use crate::prelude::*;
use crate::project::YarnProjectConfigToLoad;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use bevy::asset::FileAssetIo;
use bevy::prelude::*;
use std::path::Path;

pub(crate) fn panic_on_err(In(result): In<SystemResult>) {
    if let Err(e) = result {
        panic!("Error in Yarn Slinger plugin: {e}");
    }
}

pub(crate) fn in_development(
    project: Option<Res<YarnProject>>,
    project_to_load: Option<Res<YarnProjectConfigToLoad>>,
) -> bool {
    if let Some(project) = project {
        return project.file_generation_mode == FileGenerationMode::Development;
    }
    if let Some(project_to_load) = project_to_load {
        return project_to_load.file_generation_mode == FileGenerationMode::Development;
    }
    false
}

pub(crate) fn has_localizations(
    project: Option<Res<YarnProject>>,
    project_to_load: Option<Res<YarnProjectConfigToLoad>>,
) -> bool {
    if let Some(project) = project {
        return project.localizations.is_some();
    }
    if let Some(project_to_load) = project_to_load {
        return matches!(project_to_load.localizations, Some(Some(_)));
    }
    false
}

pub(crate) fn events_in_queue<T: Event>() -> impl FnMut(EventReader<T>) -> bool + Clone {
    move |reader: EventReader<T>| !reader.is_empty()
}

pub(crate) fn get_assets_dir_path(asset_server: &AssetServer) -> Result<impl AsRef<Path> + '_> {
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    {
        let asset_io = asset_server.asset_io();
        let file_asset_io = asset_io.downcast_ref::<FileAssetIo>().context(
            "Failed to downcast asset server IO to `FileAssetIo`. \
    The vanilla Bevy `FileAssetIo` is the only one supported by Yarn Slinger",
        )?;
        Ok(file_asset_io.root_path())
    }
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    {
        let _asset_server = asset_server;
        Ok(Path::new("./assets"))
    }
}

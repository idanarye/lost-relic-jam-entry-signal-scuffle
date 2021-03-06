use bevy::prelude::*;
use bevy_yoleck::vpeol_2d::{yoleck_vpeol_position_edit_adapter, YoleckVpeolTransform2dProjection};
use bevy_yoleck::{egui, YoleckEdit, YoleckExtForApp, YoleckPopulate, YoleckTypeHandler};
use serde::{Deserialize, Serialize};

use crate::global_types::{AppState, CameraInclude, DownloadProgress, WifiClient, WifiRouter};
use crate::loading::GameAssets;

pub struct WifiPlugin;

impl Plugin for WifiPlugin {
    fn build(&self, app: &mut App) {
        app.add_yoleck_handler({
            YoleckTypeHandler::<Wifi>::new("Wifi")
                .populate_with(populate)
                .with(yoleck_vpeol_position_edit_adapter(|wifi: &mut Wifi| {
                    YoleckVpeolTransform2dProjection {
                        translation: &mut wifi.position,
                    }
                }))
                .edit_with(edit)
        });
        app.add_system_set({
            SystemSet::on_update(AppState::Game)
                .with_system(update_access_points)
                .with_system(update_download_progress)
        });
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Wifi {
    #[serde(default)]
    position: Vec2,
    #[serde(default)]
    full_strengh_radius: f32,
}

fn populate(mut populate: YoleckPopulate<Wifi>, game_assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.insert(WifiRouter {
            full_strengh_radius: data.full_strengh_radius,
        });
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(1.0, 1.0)),
                color: Color::rgba(1.0, 1.0, 1.0, 0.9),
                ..Default::default()
            },
            texture: game_assets.wifi.clone(),
            ..Default::default()
        });
        cmd.insert_bundle(TransformBundle::from_transform(
            Transform::from_translation(data.position.extend(1.0)),
        ));
        cmd.insert(CameraInclude);
    });
}

fn edit(mut edit: YoleckEdit<Wifi>) {
    edit.edit(|_, data, ui| {
        ui.add({
            egui::Slider::new(&mut data.full_strengh_radius, 0.0..=4.0)
                .prefix("Full Strength Radius: ")
        });
    });
}

fn update_access_points(
    mut clients_query: Query<(&GlobalTransform, &mut WifiClient)>,
    wifis_query: Query<(Entity, &GlobalTransform, &WifiRouter)>,
) {
    for (client_transform, mut client) in clients_query.iter_mut() {
        if let Some((wifi_entity, signal_strength)) = wifis_query
            .iter()
            .map(|(wifi_entity, wifi_transform, wifi_router)| {
                let distance_sq = client_transform
                    .translation
                    .distance_squared(wifi_transform.translation);
                let distance_sq = distance_sq - wifi_router.full_strengh_radius.powi(2);
                let distance_sq = distance_sq.max(0.0);
                let signal_strength = 1.0 / (1.0 + (0.2 * distance_sq).ln_1p());
                (wifi_entity, signal_strength)
            })
            .max_by_key(|(_, signal_strength)| float_ord::FloatOrd(*signal_strength))
        {
            client.access_point = Some(wifi_entity);
            client.signal_strength = signal_strength;
        } else {
            client.access_point = None;
            client.signal_strength = 0.0;
        }
    }
}

fn update_download_progress(
    time: Res<Time>,
    mut query: Query<(&WifiClient, &mut DownloadProgress)>,
) {
    for (wifi_client, mut download_progress) in query.iter_mut() {
        let connected = 0.8 <= wifi_client.signal_strength;
        *download_progress = match *download_progress {
            DownloadProgress::Disconnected => {
                if connected {
                    DownloadProgress::Downloading { progress: 0.0 }
                } else {
                    DownloadProgress::Disconnected
                }
            }
            DownloadProgress::LosingConnection {
                time_before_disconnection,
                progress,
            } => {
                if connected {
                    DownloadProgress::Downloading { progress }
                } else {
                    let time_before_disconnection =
                        time_before_disconnection - time.delta_seconds();
                    if time_before_disconnection <= 0.0 {
                        DownloadProgress::Disconnected
                    } else {
                        DownloadProgress::LosingConnection {
                            time_before_disconnection,
                            progress,
                        }
                    }
                }
            }
            DownloadProgress::Downloading { progress } => {
                if connected {
                    let progress = progress + time.delta_seconds() / 10.0;
                    if 1.0 <= progress {
                        DownloadProgress::Completed
                    } else {
                        DownloadProgress::Downloading { progress }
                    }
                } else {
                    DownloadProgress::LosingConnection {
                        time_before_disconnection: 5.0,
                        progress,
                    }
                }
            }
            DownloadProgress::Completed => DownloadProgress::Completed,
        };
    }
}

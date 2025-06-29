// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::cache_update_handler::CacheUpdateHandler;
use crate::tx_handler::TxHandler;
use sui_json_rpc_types::SuiEvent;
use sui_types::base_types::ObjectID;
use sui_types::effects::TransactionEffects;
use sui_types::object::Object;
use tokio::sync::mpsc;
use tracing::debug;

/// Background task types that can be processed asynchronously
#[derive(Debug)]
pub enum BackgroundTask {
    /// Cache update task for changed objects
    CacheUpdate {
        changed_objects: Vec<(ObjectID, Object)>,
    },
    /// Transaction effects and events processing task
    TxEffectsAndEvents {
        effects: TransactionEffects,
        events: Vec<SuiEvent>,
    },
}

/// Background task processor that handles async tasks in a dedicated worker
#[derive(Clone)]
pub struct BackgroundTaskProcessor {
    task_sender: mpsc::UnboundedSender<BackgroundTask>,
}

impl BackgroundTaskProcessor {
    /// Create a new background task processor
    pub fn new(cache_handler: CacheUpdateHandler, tx_handler: TxHandler) -> Self {
        let (task_sender, mut task_receiver) = mpsc::unbounded_channel::<BackgroundTask>();

        // Spawn the background worker task
        tokio::spawn(async move {
            debug!("Background task processor started");
            
            while let Some(task) = task_receiver.recv().await {
                Self::process_task(task, &cache_handler, &tx_handler).await;
            }
            
            debug!("Background task processor stopped");
        });

        Self { task_sender }
    }

    /// Submit cache update task
    pub fn submit_cache_update(&self, changed_objects: Vec<(ObjectID, Object)>) {
        let task = BackgroundTask::CacheUpdate { changed_objects };
        let _ = self.task_sender.send(task);
    }

    /// Submit tx effects and events task
    pub fn submit_tx_effects_and_events(&self, effects: TransactionEffects, events: Vec<SuiEvent>) {
        let task = BackgroundTask::TxEffectsAndEvents { effects, events };
        let _ = self.task_sender.send(task);
    }

    /// Process a single background task
    async fn process_task(
        task: BackgroundTask,
        cache_handler: &CacheUpdateHandler,
        tx_handler: &TxHandler,
    ) {
        match task {
            BackgroundTask::CacheUpdate { changed_objects } => {
                debug!("Processing cache update for {} objects", changed_objects.len());
                cache_handler.notify_written(changed_objects).await;
            }
            
            BackgroundTask::TxEffectsAndEvents { effects, events } => {
                debug!("Processing tx effects and {} events", events.len());
                let _ = tx_handler.send_tx_effects_and_events(&effects, events).await;
            }
        }
    }
}

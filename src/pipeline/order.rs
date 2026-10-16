//! Die Reihenfolge der Behaviors in der Pipeline.

use std::cmp::Reverse;
use std::sync::Arc;

use crate::pipeline::{Pipeline, Stage, StreamPipeline};
use crate::wrappers::{BehaviorWrapper, StreamBehaviorWrapper};
use crate::{Request, StreamRequest};

/// Ordnet `behaviors`, die in Registrierungsreihenfolge mit ihrer [`Stage`] kommen, für die Pipeline: das innerste zuerst, jedes, wo seine Stufe sagt.
pub(crate) fn behaviors<R: Request>(
    mut behaviors: Vec<(Stage, Arc<BehaviorWrapper<R>>)>,
) -> Pipeline<R> {
    // Innerste Stufe zuerst; die Sortierung ist stabil, in jeder Stufe bleibt die Registrierungsreihenfolge.
    behaviors.sort_by_key(|(stage, _)| Reverse(*stage));
    // Bei `Around` und `Before` liegt das zuerst registrierte außen, also hinten.
    for stage in [Stage::Around, Stage::Before] {
        let start = behaviors.partition_point(|(other, _)| *other > stage);
        let end = behaviors.partition_point(|(other, _)| *other >= stage);
        behaviors
            .get_mut(start..end)
            .into_iter()
            .for_each(<[_]>::reverse);
    }
    behaviors
        .into_iter()
        .map(|(_, behavior)| behavior)
        .collect()
}

/// Ordnet `behaviors`, die in Registrierungsreihenfolge kommen, für die Stream-Pipeline: das innerste zuerst, das zuerst registrierte liegt außen.
pub(crate) fn stream_behaviors<R: StreamRequest>(
    behaviors: Vec<Arc<StreamBehaviorWrapper<R>>>,
) -> StreamPipeline<R> {
    behaviors.into_iter().rev().collect()
}

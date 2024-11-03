//!
use std::collections::VecDeque;

use bones_lib::{ecs::World, prelude::default};

/// Max frames of data in desync history buffer - this is set to match `ggrs::MAX_CHECKSUM_HISTORY_SIZE`,
/// but is private so cannot be used directly.
const MAX_DESYNC_HISTORY_BUFFER: usize = 32;

/// Settings for desync detection
#[derive(Clone)]
pub struct DetectDesyncs {
    /// Interval in frames of how often to hash state and check for desync with other clients.
    /// i.e if set to 10, will check every 10th frame.
    pub detection_interval: u32,

    /// Override of hash function used to hash world for desync detection.
    /// By default, [`World`]'s [`DesyncHash`] impl is used.
    pub world_hash_func: Option<fn(&World) -> u64>,

    /// When using feature `desync-debug`, a [`DesyncTree`] will be built. Resources and Components
    /// that do not support hashing can be optionally included in tree to help highlight candidates
    /// to be opted into desync-detection.
    pub include_unhashable_nodes: bool,
}

impl Default for DetectDesyncs {
    fn default() -> Self {
        Self {
            detection_interval: 60,
            world_hash_func: None,
            include_unhashable_nodes: false,
        }
    }
}
/// Store history of desync detection data, such as a [`DesyncTree`]. When ggrs finds a desync in past,
/// we can retrieve this data for debugging. Ggrs has a fixed limit of pending desync frames it tests,
/// so we match it by keeping the last [`MAX_DESYNC_HISTORY_BUFFER`] of frame data at the desync detect interval.
///
/// Desync data provided in `record` will only be saved if frame coincides with desync detect interval, otherwise
/// ggrs will never test this frame, and we do not need to buffer it.
pub struct DesyncDebugHistoryBuffer<T> {
    buffer: VecDeque<(u32, T)>,

    /// Desync detection interval, should match ggrs session config.
    desync_detect_interval: u32,
}

impl<T> DesyncDebugHistoryBuffer<T> {
    /// Create buffer, use same desync detect interval configured on ggrs session.
    pub fn new(desync_detect_interval: u32) -> Self {
        Self {
            desync_detect_interval,
            buffer: default(),
        }
    }

    /// Check if this frame coincides with desync detection interval.
    /// If not, we will not perform desync checks on it, and do not need to record history for frame.
    pub fn is_desync_detect_frame(&self, frame: u32) -> bool {
        // GGRS sends desync detections every X frames where X is interval, and first frame is interval.
        frame % self.desync_detect_interval == 0
    }

    /// Get desync data for frame if it is available.
    pub fn get_frame_data(&self, frame: u32) -> Option<&T> {
        // Don't bother looking for data if not a desync detect frame
        if !self.is_desync_detect_frame(frame) {
            return None;
        }

        self.buffer.iter().find_map(|d| {
            if d.0 == frame {
                return Some(&d.1);
            }
            None
        })
    }

    /// Possibly record frame and desync data. It is only recorded on frames matching
    /// desync detect interval, as ggrs will not check for desyns otherwise and we don't
    /// need to save it.
    pub fn record(&mut self, frame: u32, desync_data: T) {
        // Only record if on a frame that will be desync detected.
        if self.is_desync_detect_frame(frame) {
            while self.buffer.len() >= MAX_DESYNC_HISTORY_BUFFER {
                self.buffer.pop_front();
            }

            self.buffer.push_back((frame, desync_data));
        }
    }
}

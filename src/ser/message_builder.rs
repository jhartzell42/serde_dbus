use std::cmp::max;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::align::align;

pub(super) struct PendingMessage {
    pub builder: MessageBuilder,
    pub signature: Vec<u8>,
}

impl Default for PendingMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingMessage {
    pub fn new() -> Self {
        Self {
            builder: MessageBuilder::new(),
            signature: Vec::new(),
        }
    }
}

// This is probably the most performance-critical and most performance-damaging component.
// Currently it is written to make a lot of allocations, but that can be optimized later
// without breaking its contract with the outside.

pub(super) struct MessageBuilder {
    data: Vec<MessageComponent>, // Invariant: top one is always an alignment slice
}

fn align_vec(vec: &mut Vec<u8>, alignment: usize) {
    vec.resize(align(vec.len(), alignment), 0);
}

// TODO: use a more performant, less allocation-heavy data structure for building up this information
#[derive(Debug, Clone, PartialEq)]
struct AlignmentSlice {
    alignment: usize,
    data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LengthToken(usize);

impl LengthToken {
    fn next() -> LengthToken {
        static GLOBAL_LENGTH_TOKEN_COUNT: AtomicUsize = AtomicUsize::new(0);
        LengthToken(GLOBAL_LENGTH_TOKEN_COUNT.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum MessageComponent {
    AlignmentSlice(AlignmentSlice),
    LengthBegin(LengthToken),
    LengthEnd(LengthToken),
}

impl MessageBuilder {
    fn top(&mut self) -> &mut AlignmentSlice {
        if let MessageComponent::AlignmentSlice(ref mut a_slice) = self
            .data
            .last_mut()
            .expect("always at least one alignment slice")
        {
            a_slice
        } else {
            panic!("top message component must always be an alignment slice");
        }
    }

    pub(super) fn start_length(&mut self) -> LengthToken {
        let LengthToken(token) = LengthToken::next();
        self.align(4);
        self.data
            .push(MessageComponent::LengthBegin(LengthToken(token)));
        self.data
            .push(MessageComponent::AlignmentSlice(AlignmentSlice {
                alignment: 4,
                data: Vec::new(),
            }));
        LengthToken(token)
    }

    pub(super) fn finish_length(&mut self, token: LengthToken) {
        self.data.push(MessageComponent::LengthEnd(token));
        self.data
            .push(MessageComponent::AlignmentSlice(AlignmentSlice {
                alignment: 1,
                data: Vec::new(),
            }));
    }

    // Note: alignment must be power of 2
    pub(super) fn align(&mut self, alignment: usize) {
        {
            let top = self.top();
            if top.data.is_empty() {
                // We potentially need to increase the alignment guarantee
                // of this segment.
                top.alignment = max(top.alignment, alignment);
                return;
            } else if top.alignment >= alignment {
                // We already have the guarantee we need.
                // Just align within the data.
                align_vec(&mut top.data, alignment);
                return;
            }
        }

        // Need new alignment guarantee. None of the existing alignment guarantees
        // can guarantee this, but we cannot more strictly align previously outputted
        // data.
        //
        // This is a serious flaw with the DBus format, that we cannot know ahead of
        // time how much padding is actually required.
        self.data
            .push(MessageComponent::AlignmentSlice(AlignmentSlice {
                alignment,
                data: Vec::new(),
            }));
    }

    // This does not touch self's signature at all
    pub(super) fn append_data(&mut self, other: &MessageBuilder) {
        for slice in &other.data {
            match slice {
                MessageComponent::AlignmentSlice(slice) => {
                    self.align(slice.alignment);
                    let out = self.prepare_write(slice.data.len());
                    out.copy_from_slice(&slice.data);
                }
                other => {
                    self.data.push(other.clone());
                    self.data
                        .push(MessageComponent::AlignmentSlice(AlignmentSlice {
                            alignment: 1,
                            data: Vec::new(),
                        }));
                }
            }
        }
    }

    // TODO: The interface of this function seems certainly wrong.
    // I'd like to replace it with something that adds a `&[u8]`, but
    // that is a task relatively low down on the ol' priority list.
    pub(super) fn prepare_write(&mut self, size: usize) -> &mut [u8] {
        let top = self.top();
        let old_len = top.data.len();
        let new_len = old_len + size;
        top.data.resize(new_len, 0);
        &mut top.data[old_len..new_len]
    }

    pub(super) fn new() -> Self {
        Self {
            data: vec![MessageComponent::AlignmentSlice(AlignmentSlice {
                alignment: 1usize,
                data: Vec::new(),
            })],
        }
    }

    pub(super) fn complete(self) -> Vec<u8> {
        let mut output_data = Vec::new();

        // This is for arrays we are currently in, where the length
        // must be backfilled after we've outputted the other data.
        let mut lengths: BTreeMap<usize, (usize, usize)> = BTreeMap::new();

        // This is for an array we have just started, and have not
        // yet determined the beginning of the array, which is considered
        // to be after any padding.
        let mut recent_length = None;

        for datum in self.data {
            match datum {
                MessageComponent::AlignmentSlice(mut a_slice) => {
                    // Now we know how many bytes this alignment will
                    // actually take, whereas before it depended on earlier
                    // context.
                    align_vec(&mut output_data, a_slice.alignment);

                    // Adjacent alignment adjustments are always
                    // consolidated, so we know we have skipped any
                    // padding. If we have just started an array,
                    // the byte count can start now.
                    if let Some(recent_length) = recent_length.take() {
                        if let Some(length_item) = lengths.get_mut(&recent_length) {
                            let new_length_begin = output_data.len();
                            length_item.1 = new_length_begin;
                        }
                    }

                    output_data.append(&mut a_slice.data);
                }
                MessageComponent::LengthBegin(LengthToken(token)) => {
                    // We are about to start an array, and need to know
                    // how long it will be, but can't until we reach
                    // the end.
                    lengths.insert(token, (output_data.len(), output_data.len() + 4));
                    output_data.extend_from_slice(&[0u8, 0u8, 0u8, 0u8]);
                    recent_length = Some(token);
                }
                MessageComponent::LengthEnd(LengthToken(token)) => {
                    // Now is the time to backfill the array length.
                    // This message component does not append to the
                    // message, but rather just backfills a length
                    // started earlier.
                    let (fill_ix, begin_ix) = *lengths
                        .get(&token)
                        .expect("length end found without matching length begin");
                    let end_ix = output_data.len();
                    let length = end_ix - begin_ix;
                    let length = length as u32;
                    let fill_in_range = &mut output_data[fill_ix..fill_ix + 4];
                    fill_in_range.copy_from_slice(&length.to_le_bytes());
                    lengths.remove(&token);
                    recent_length = None;
                }
            }
        }

        output_data
    }
}

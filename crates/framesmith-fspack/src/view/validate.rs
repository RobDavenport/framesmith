//! Validate nested references once at the binary trust boundary.
use super::*;

impl PackView<'_> {
    fn checked_range(
        &self,
        kind: u32,
        off: usize,
        count: usize,
        stride: usize,
    ) -> Result<(), Error> {
        if count == 0 {
            return Ok(());
        }
        let section = self.get_section(kind).ok_or(Error::InvalidFormat)?;
        let end = off
            .checked_add(count.checked_mul(stride).ok_or(Error::OutOfBounds)?)
            .ok_or(Error::OutOfBounds)?;
        if !off.is_multiple_of(stride) || end > section.len() {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }

    fn checked_string(&self, record: &[u8], off: usize) -> Result<(), Error> {
        self.string(
            read_u32_le(record, off).ok_or(Error::InvalidFormat)?,
            read_u16_le(record, off + 4).ok_or(Error::InvalidFormat)?,
        )
        .ok_or(Error::InvalidFormat)
        .map(|_| ())
    }

    fn checked_properties(&self, data: &[u8], state: bool) -> Result<(), Error> {
        let stride = if self.has_schema() {
            SCHEMA_PROP_SIZE
        } else {
            CHARACTER_PROP_SIZE
        };
        if !data.len().is_multiple_of(stride) {
            return Err(Error::InvalidFormat);
        }
        for prop in data.chunks_exact(stride) {
            let (tag, value) = if let Some(schema) = self.schema() {
                let id = read_u16_le(prop, 0).ok_or(Error::InvalidFormat)? as usize;
                let count = if state {
                    schema.state_prop_count()
                } else {
                    schema.char_prop_count()
                };
                if id >= count {
                    return Err(Error::InvalidFormat);
                }
                (prop[2], 4)
            } else {
                self.checked_string(prop, 0)?;
                (prop[6], 8)
            };
            match tag {
                0 => {}
                1 if read_u32_le(prop, value).ok_or(Error::InvalidFormat)? <= 1 => {}
                2 => {
                    self.string(
                        u32::from(read_u16_le(prop, value).ok_or(Error::InvalidFormat)?),
                        read_u16_le(prop, value + 2).ok_or(Error::InvalidFormat)?,
                    )
                    .ok_or(Error::InvalidFormat)?;
                }
                _ => return Err(Error::InvalidFormat),
            }
        }
        Ok(())
    }

    pub(super) fn validate_references(&self) -> Result<(), Error> {
        let state_count = self.states().map_or(0, |states| states.len());
        let u16_at = |bytes: &[u8], off| {
            read_u16_le(bytes, off)
                .map(usize::from)
                .ok_or(Error::InvalidFormat)
        };
        let u32_at = |bytes: &[u8], off| {
            read_u32_le(bytes, off)
                .map(|n| n as usize)
                .ok_or(Error::InvalidFormat)
        };
        for (kind, stride) in [
            (SECTION_MESH_KEYS, STRREF_SIZE),
            (SECTION_KEYFRAMES_KEYS, STRREF_SIZE),
            (SECTION_STATE_TAGS, STRREF_SIZE),
            (SECTION_RESOURCE_DEFS, RESOURCE_DEF_SIZE),
            (SECTION_MOVE_RESOURCE_COSTS, MOVE_RESOURCE_COST_SIZE),
            (
                SECTION_MOVE_RESOURCE_PRECONDITIONS,
                MOVE_RESOURCE_PRECONDITION_SIZE,
            ),
            (SECTION_MOVE_RESOURCE_DELTAS, MOVE_RESOURCE_DELTA_SIZE),
        ] {
            if let Some(data) = self.get_section(kind) {
                for record in data.chunks_exact(stride) {
                    self.checked_string(record, 0)?;
                }
            }
        }
        if let Some(states) = self.states() {
            for index in 0..states.len() {
                let state = states.get(index).ok_or(Error::InvalidFormat)?;
                if state.state_id() as usize != index {
                    return Err(Error::InvalidFormat);
                }
                for (key, kind) in [
                    (state.mesh_key(), SECTION_MESH_KEYS),
                    (state.keyframes_key(), SECTION_KEYFRAMES_KEYS),
                ] {
                    if key != KEY_NONE {
                        self.checked_range(kind, key as usize * STRREF_SIZE, 1, STRREF_SIZE)?;
                    }
                }
                let (off, count) = (state.hit_windows_off(), state.hit_windows_len());
                self.checked_range(
                    SECTION_HIT_WINDOWS,
                    off as usize,
                    count as usize,
                    HIT_WINDOW_SIZE,
                )?;
                let (off, count) = (state.hurt_windows_off(), state.hurt_windows_len());
                self.checked_range(
                    SECTION_HURT_WINDOWS,
                    off as usize,
                    count as usize,
                    HURT_WINDOW_SIZE,
                )?;
                let (off, count) = (state.push_windows_off(), state.push_windows_len());
                self.checked_range(
                    SECTION_PUSH_WINDOWS,
                    off as usize,
                    count as usize,
                    PUSH_WINDOW_SIZE,
                )?;
            }
        }
        for (kind, stride, off) in [
            (SECTION_HIT_WINDOWS, HIT_WINDOW_SIZE, 12),
            (SECTION_HURT_WINDOWS, HURT_WINDOW_SIZE, 4),
            (SECTION_PUSH_WINDOWS, PUSH_WINDOW_SIZE, 4),
        ] {
            if let Some(data) = self.get_section(kind) {
                for record in data.chunks_exact(stride) {
                    self.checked_range(
                        SECTION_SHAPES,
                        u32_at(record, off)?,
                        u16_at(record, off + 4)?,
                        SHAPE_SIZE,
                    )?;
                }
            }
        }
        if let Some(data) = self.get_section(SECTION_STATE_EXTRAS) {
            if data.len() / STATE_EXTRAS_SIZE != state_count {
                return Err(Error::InvalidFormat);
            }
            for record in data.chunks_exact(STATE_EXTRAS_SIZE) {
                for (off, kind, stride) in [
                    (0, SECTION_EVENT_EMITS, EVENT_EMIT_SIZE),
                    (8, SECTION_EVENT_EMITS, EVENT_EMIT_SIZE),
                    (16, SECTION_EVENT_EMITS, EVENT_EMIT_SIZE),
                    (24, SECTION_MOVE_NOTIFIES, MOVE_NOTIFY_SIZE),
                    (32, SECTION_MOVE_RESOURCE_COSTS, MOVE_RESOURCE_COST_SIZE),
                    (
                        40,
                        SECTION_MOVE_RESOURCE_PRECONDITIONS,
                        MOVE_RESOURCE_PRECONDITION_SIZE,
                    ),
                    (48, SECTION_MOVE_RESOURCE_DELTAS, MOVE_RESOURCE_DELTA_SIZE),
                    (64, SECTION_CANCELS_U16, 2),
                ] {
                    self.checked_range(
                        kind,
                        u32_at(record, off)?,
                        u16_at(record, off + 4)?,
                        stride,
                    )?;
                }
                self.checked_string(record, 56)?;
            }
        }
        if let Some(data) = self.get_section(SECTION_MOVE_NOTIFIES) {
            for record in data.chunks_exact(MOVE_NOTIFY_SIZE) {
                self.checked_range(
                    SECTION_EVENT_EMITS,
                    u32_at(record, 4)?,
                    u16_at(record, 8)?,
                    EVENT_EMIT_SIZE,
                )?;
            }
        }
        if let Some(data) = self.get_section(SECTION_EVENT_EMITS) {
            for record in data.chunks_exact(EVENT_EMIT_SIZE) {
                self.checked_string(record, 0)?;
                self.checked_range(
                    SECTION_EVENT_ARGS,
                    u32_at(record, 8)?,
                    u16_at(record, 12)?,
                    EVENT_ARG_SIZE,
                )?;
            }
        }
        if let Some(data) = self.get_section(SECTION_EVENT_ARGS) {
            for record in data.chunks_exact(EVENT_ARG_SIZE) {
                self.checked_string(record, 0)?;
                match record[8] {
                    0..=2 => {}
                    3 => self.checked_string(record, 12)?,
                    _ => return Err(Error::InvalidFormat),
                }
            }
        }
        for kind in [SECTION_CANCELS_U16, SECTION_CANCEL_DENIES] {
            if let Some(data) = self.get_section(kind) {
                for record in data.chunks_exact(2) {
                    if u16_at(record, 0)? >= state_count {
                        return Err(Error::InvalidFormat);
                    }
                }
            }
        }
        if let Some(data) = self.get_section(SECTION_CANCEL_TAG_RULES) {
            for record in data.chunks_exact(CANCEL_TAG_RULE_SIZE) {
                for off in [0, 8] {
                    if read_u32_le(record, off) == Some(u32::MAX) {
                        if u16_at(record, off + 4)? != 0 {
                            return Err(Error::InvalidFormat);
                        }
                    } else {
                        self.checked_string(record, off)?;
                    }
                }
            }
        }
        if let Some(data) = self.get_section(SECTION_STATE_TAG_RANGES) {
            if data.len() / STRREF_SIZE != state_count {
                return Err(Error::InvalidFormat);
            }
            for record in data.chunks_exact(STRREF_SIZE) {
                self.checked_range(
                    SECTION_STATE_TAGS,
                    u32_at(record, 0)?,
                    u16_at(record, 4)?,
                    STRREF_SIZE,
                )?;
            }
        }
        if let Some(data) = self.get_section(SECTION_SCHEMA) {
            let count = u16_at(data, 0)? + u16_at(data, 2)? + u16_at(data, 4)?;
            if data.len() != SCHEMA_HEADER_SIZE + count * STRREF_SIZE {
                return Err(Error::InvalidFormat);
            }
            for record in data[SCHEMA_HEADER_SIZE..].chunks_exact(STRREF_SIZE) {
                self.checked_string(record, 0)?;
            }
        }
        if let Some(data) = self.get_section(SECTION_CHARACTER_PROPS) {
            self.checked_properties(data, false)?;
        }
        if let Some(data) = self.get_section(SECTION_STATE_PROPS) {
            let index_size = state_count
                .checked_mul(STATE_PROPS_INDEX_ENTRY_SIZE)
                .ok_or(Error::OutOfBounds)?;
            if data.len() < index_size {
                return Err(Error::InvalidFormat);
            }
            for index in 0..state_count {
                let len = u16_at(data, index * STATE_PROPS_INDEX_ENTRY_SIZE + 4)?;
                if len != 0 {
                    self.checked_properties(
                        self.state_props_raw(index).ok_or(Error::InvalidFormat)?,
                        true,
                    )?;
                }
            }
        }
        Ok(())
    }
}

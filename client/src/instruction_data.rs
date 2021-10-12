use std::io::Write;

use anchor_client::anchor_lang::{AnchorSerialize, InstructionData};

pub struct DynamicInstructionData {
    data: Vec<u8>,
}

pub fn dynamic<T: InstructionData>(id: T) -> DynamicInstructionData {
    DynamicInstructionData { data: id.data() }
}

impl AnchorSerialize for DynamicInstructionData {
    fn serialize<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        todo!()
    }
}

impl InstructionData for DynamicInstructionData {
    fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

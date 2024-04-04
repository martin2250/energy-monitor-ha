use crate::stpm::driver::StpmDriver;

use super::{registers::Reg, Stpm};

enum ReaderOutput<'r> {
    None,
    Signed(&'r mut i32),
    Unsigned(&'r mut u32),
}

pub struct Reader<'r, 'a, D: StpmDriver> {
    output: ReaderOutput<'r>,
    stpm: &'r mut Stpm<'a, D>,
}

impl<'r, 'a, D> Reader<'r, 'a, D>
where
    D: StpmDriver,
    'a: 'r,
{
    pub fn create(chip: &'r mut Stpm<'a, D>) -> Self {
        Self {
            output: ReaderOutput::None,
            stpm: chip,
        }
    }

    async fn read_internal(&mut self, reg: Option<u8>) -> Result<(), D::Error> {
        let data_read = self.stpm.driver.transaction(reg, None).await?;
        match &mut self.output {
            ReaderOutput::None => {}
            ReaderOutput::Signed(data_out) => **data_out = data_read as i32,
            ReaderOutput::Unsigned(data_out) => **data_out = data_read,
        }
        Ok(())
    }

    pub async fn read_u32(mut self, reg: Reg, output: &'r mut u32) -> Result<Self, D::Error> {
        self.read_internal(Some(reg.addr())).await?;
        self.output = ReaderOutput::Unsigned(output);
        Ok(self)
    }

    pub async fn read_i32(mut self, reg: Reg, output: &'r mut i32) -> Result<Self, D::Error> {
        self.read_internal(Some(reg.addr())).await?;
        self.output = ReaderOutput::Signed(output);
        Ok(self)
    }

    pub async fn end(mut self) -> Result<(), D::Error> {
        self.read_internal(None).await
    }
}

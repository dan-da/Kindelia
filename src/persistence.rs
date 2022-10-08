use std::io::{Read, Write, Result as IoResult, Error, ErrorKind};
use std::hash::{Hash, BuildHasher};
use std::collections::HashMap;
use std::sync::Arc;
use std::ops::Deref;
use crate::hvm::{CompFunc, Func, compile_func};
use crate::bits::ProtoSerialize;

pub trait DiskSer
where
  Self: Sized,
{
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize>;
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<Self>>;
}

impl DiskSer for u8 {
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize> {
    sink.write(&self.to_le_bytes())
  }
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<u8>> {
    let mut buf = [0; 1];
    let bytes_read = source.read(&mut buf)?;
    match bytes_read {
      0 => { Ok(None) }
      _ => { Ok(Some(u8::from_le_bytes(buf))) }
    }
  }
}
impl DiskSer for i128 {
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize> {
    sink.write(&self.to_le_bytes())
  }
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<i128>> {
    const BYTES : usize = (i128::BITS / 8) as usize;
    const AT_MOST : usize = BYTES-1;
    let mut buf = [0; BYTES];
    let bytes_read = source.read(&mut buf)?;
    match bytes_read {
      0 => { Ok(None) }
      1..=AT_MOST => { Err(Error::from(ErrorKind::UnexpectedEof)) }
      _ => { Ok(Some(i128::from_le_bytes(buf))) }
    }
  }
}

impl DiskSer for u128 {
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize> {
    sink.write(&self.to_le_bytes())
  }
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<u128>> {
    const BYTES : usize = (u128::BITS / 8) as usize;
    const AT_MOST : usize = BYTES-1;
    let mut buf = [0; BYTES];
    let bytes_read = source.read(&mut buf)?;
    match bytes_read {
      0 => { Ok(None) }
      1..=AT_MOST => { Err(Error::from(ErrorKind::UnexpectedEof)) }
      _ => { Ok(Some(u128::from_le_bytes(buf))) }
    }
  }
}

// we assume that every map will be stored in a whole file.
// because of that, it will consume all of the file while reading it.
impl<K, V, H> DiskSer for HashMap<K, V, H>
where
  K: DiskSer + Eq + Hash,
  V: DiskSer,
  H: BuildHasher + Default,
{
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize> {
    let mut total_written = 0;
    for (k, v) in self {
      let key_size = k.disk_serialize(sink)?;
      let val_size = v.disk_serialize(sink)?;
      total_written += key_size + val_size;
    }
    sink.flush()?;
    Ok(total_written)
  }
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<Self>> {
    let mut slf = HashMap::with_hasher(H::default());
    while let Some(key) = K::disk_deserialize(source)? {
      let val = V::disk_deserialize(source)?;
      if let Some(val) = val {
        slf.insert(key, val);
      }
      else {
        return Err(Error::from(ErrorKind::UnexpectedEof));
      }     
    }
    Ok(Some(slf))
  }
}

impl <K> DiskSer for Vec<K>
where
  K: DiskSer,
{
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize> {
    let mut total_written = 0;
    for elem in self {
      let elem_size = elem.disk_serialize(sink)?;
      total_written += elem_size;
    }
    Ok(total_written)      
  }
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<Self>> {
    let mut res = Vec::new();
    while let Some(elem) = K::disk_deserialize(source)? {
        res.push(elem);
    }
    Ok(Some(res))
  }
}

impl<T> DiskSer for Arc<T>
where
  T: DiskSer,
{
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize> {
    let t = Arc::deref(self);
    t.disk_serialize(sink)
  }
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<Self>> {
    let t = T::disk_deserialize(source)?;
    Ok(t.map(Arc::new))
  }
}

impl DiskSer for CompFunc {
  fn disk_serialize<W: Write>(&self, sink: &mut W) -> IoResult<usize>{
    let func_buff = self.func.proto_serialized().to_bytes();
    let size = func_buff.len() as u128;
    let written1 = size.disk_serialize(sink)?;
    let written2 = func_buff.disk_serialize(sink)?;
    Ok(written1 + written2)
  }
  fn disk_deserialize<R: Read>(source: &mut R) -> IoResult<Option<Self>> {
    // let compfunc = CompFunc {};
    if let Some(len) = u128::disk_deserialize(source)? {
      let len = len as usize;
      let mut buf = vec![0; len];
      let read_bytes = source.read(&mut buf)?;
      if read_bytes != len {
        return Err(Error::from(ErrorKind::UnexpectedEof));
      }
      let func = &Func::proto_deserialized(&bit_vec::BitVec::from_bytes(&buf))
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?; // invalid data? which error is better?
      let func = compile_func(func, false)
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
      Ok(Some(func))
    }
    else {
      Ok(None)
    }
  }
}

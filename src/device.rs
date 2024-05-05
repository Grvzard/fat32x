use std::io::{Read, Seek};

pub(crate) trait Device: Seek + Read {}

impl Device for std::fs::File {}

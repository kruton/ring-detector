// MIT License
//
// Copyright (c) 2018 sorz
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use log::warn;
use std::{fmt, fs::remove_file, path::Path};

/// File on this path will be removed on `drop()`.
pub struct AutoRemoveFile<'a> {
    path: &'a str,
    auto_remove: bool,
}

impl AutoRemoveFile<'_> {
    pub fn set_auto_remove(&mut self, enable: bool) {
        self.auto_remove = enable;
    }
}

impl<'a> From<&'a str> for AutoRemoveFile<'a> {
    fn from(path: &'a str) -> Self {
        AutoRemoveFile {
            path,
            auto_remove: false,
        }
    }
}

impl<'a> Drop for AutoRemoveFile<'a> {
    fn drop(&mut self) {
        if self.auto_remove {
            if let Err(err) = remove_file(&self.path) {
                warn!("fail to remove {}: {}", self.path, err);
            }
        }
    }
}

impl<'a> AsRef<Path> for &'a AutoRemoveFile<'a> {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl fmt::Display for AutoRemoveFile<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

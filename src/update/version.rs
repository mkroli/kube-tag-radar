/*
 * Copyright 2025 Michael Krolikowski
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use oci_client::Reference;

use crate::database::Image;
use anyhow::Result;

pub trait ImageVersion {
    fn image_version(&self) -> Result<Option<String>>;
}

impl ImageVersion for Image {
    fn image_version(&self) -> Result<Option<String>> {
        let reference = self.image.parse::<Reference>()?;
        let version = reference.tag().map(|tag| tag.to_string());
        Ok(version)
    }
}

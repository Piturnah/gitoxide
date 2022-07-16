use git_features::threading::OwnShared;
use std::borrow::Cow;

use crate::{
    file::{self, rename_section, SectionMut},
    lookup,
    parse::section,
    File,
};

/// Mutating low-level access methods.
impl<'event> File<'event> {
    /// Returns an mutable section with a given name and optional subsection.
    pub fn section_mut<'a>(
        &'a mut self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&str>,
    ) -> Result<SectionMut<'a, 'event>, lookup::existing::Error> {
        let id = self
            .section_ids_by_name_and_subname(section_name.as_ref(), subsection_name)?
            .rev()
            .next()
            .expect("BUG: Section lookup vec was empty");
        Ok(SectionMut::new(
            self.sections
                .get_mut(&id)
                .expect("BUG: Section did not have id from lookup"),
        ))
    }

    /// Adds a new section. If a subsection name was provided, then
    /// the generated header will use the modern subsection syntax.
    /// Returns a reference to the new section for immediate editing.
    ///
    /// # Examples
    ///
    /// Creating a new empty section:
    ///
    /// ```
    /// # use git_config::File;
    /// # use std::convert::TryFrom;
    /// let mut git_config = git_config::File::default();
    /// let _section = git_config.new_section("hello", Some("world".into()));
    /// assert_eq!(git_config.to_string(), "[hello \"world\"]\n");
    /// ```
    ///
    /// Creating a new empty section and adding values to it:
    ///
    /// ```
    /// # use git_config::File;
    /// # use std::convert::TryFrom;
    /// # use bstr::ByteSlice;
    /// # use git_config::parse::section;
    /// let mut git_config = git_config::File::default();
    /// let mut section = git_config.new_section("hello", Some("world".into()))?;
    /// section.push(section::Key::try_from("a")?, "b");
    /// assert_eq!(git_config.to_string(), "[hello \"world\"]\n\ta = b\n");
    /// let _section = git_config.new_section("core", None);
    /// assert_eq!(git_config.to_string(), "[hello \"world\"]\n\ta = b\n[core]\n");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new_section(
        &mut self,
        name: impl Into<Cow<'event, str>>,
        subsection: impl Into<Option<Cow<'event, str>>>,
    ) -> Result<SectionMut<'_, 'event>, section::header::Error> {
        let mut section =
            self.push_section_internal(file::Section::new(name, subsection, OwnShared::clone(&self.meta))?);
        section.push_newline();
        Ok(section)
    }

    /// Removes the section, returning the events it had, if any. If multiple
    /// sections have the same name, then the last one is returned. Note that
    /// later sections with the same name have precedent over earlier ones.
    ///
    /// # Examples
    ///
    /// Creating and removing a section:
    ///
    /// ```
    /// # use git_config::File;
    /// # use std::convert::TryFrom;
    /// let mut git_config = git_config::File::try_from(
    /// r#"[hello "world"]
    ///     some-value = 4
    /// "#)?;
    ///
    /// let events = git_config.remove_section("hello", Some("world".into()));
    /// assert_eq!(git_config.to_string(), "");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Precedence example for removing sections with the same name:
    ///
    /// ```
    /// # use git_config::File;
    /// # use std::convert::TryFrom;
    /// let mut git_config = git_config::File::try_from(
    /// r#"[hello "world"]
    ///     some-value = 4
    /// [hello "world"]
    ///     some-value = 5
    /// "#)?;
    ///
    /// let events = git_config.remove_section("hello", Some("world".into()));
    /// assert_eq!(git_config.to_string(), "[hello \"world\"]\n    some-value = 4\n");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove_section<'a>(
        &mut self,
        section_name: &str,
        subsection_name: impl Into<Option<&'a str>>,
    ) -> Option<file::section::Body<'event>> {
        let id = self
            .section_ids_by_name_and_subname(section_name, subsection_name.into())
            .ok()?
            .rev()
            .next()?;
        self.section_order.remove(
            self.section_order
                .iter()
                .position(|v| *v == id)
                .expect("known section id"),
        );
        self.sections.remove(&id).map(|s| s.body)
    }

    /// Adds the provided section to the config, returning a mutable reference
    /// to it for immediate editing.
    /// Note that its meta-data will remain as is.
    pub fn push_section(
        &mut self,
        section: file::Section<'event>,
    ) -> Result<SectionMut<'_, 'event>, section::header::Error> {
        Ok(self.push_section_internal(section))
    }

    /// Renames a section, modifying the last matching section.
    pub fn rename_section<'a>(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl Into<Option<&'a str>>,
        new_section_name: impl Into<Cow<'event, str>>,
        new_subsection_name: impl Into<Option<Cow<'event, str>>>,
    ) -> Result<(), rename_section::Error> {
        let id = self
            .section_ids_by_name_and_subname(section_name.as_ref(), subsection_name.into())?
            .rev()
            .next()
            .expect("list of sections were empty, which violates invariant");
        let section = self.sections.get_mut(&id).expect("known section-id");
        section.header = section::Header::new(new_section_name, new_subsection_name)?;
        Ok(())
    }
}

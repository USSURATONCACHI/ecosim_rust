#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SameIncludes {
    IgnoreAll,
    DeleteRepeats,
    ThrowAnError,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MultipleVersions {
    IgnoreAll,
    SetToHighest,
    SetToLowest,
    SetToFirst,
    SetToLast,
    ThrowAnError,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VersionNotAtTheBeginning {
    Ignore,
    MoveToBeginning,
    ThrowAnError,
}

#[derive(Debug, Clone)]
pub struct Rule<T: Copy> {
    is_default: bool,
    rule: T,
}
impl<T: Copy> Rule<T> {
    fn default(rule: T) -> Self {
        Self {
            is_default: true,
            rule
        }
    }
    fn merge(self, new: Rule<T>) -> Rule<T> {
        match new.is_default {
            false => new,
            true => match self.is_default {
                true => new,
                false => self,
            },
        }
    }
    fn add(&mut self, new: Rule<T>) {
        if !new.is_default || self.is_default {
            self.is_default = new.is_default;
            self.rule = new.rule;
        }
    }
    pub fn value(&self) -> T {
        self.rule
    }
}

#[derive(Debug, Clone)]
pub struct ParseRules {
    display_warns:          Rule<bool>,
    same_includes:          Rule<SameIncludes>,
    multiple_versions:      Rule<MultipleVersions>,
    version_natb:           Rule<VersionNotAtTheBeginning>,
}
impl ParseRules {
    pub fn new() -> ParseRules {
        ParseRules {
            display_warns: Rule::default(true),
            same_includes: Rule::default(SameIncludes::DeleteRepeats),
            multiple_versions: Rule::default(MultipleVersions::SetToHighest),
            version_natb: Rule::default(VersionNotAtTheBeginning::MoveToBeginning),
        }
    }

    pub fn merge(self, new_rules: ParseRules) -> ParseRules {
        ParseRules {
            display_warns: self.display_warns.merge(new_rules.display_warns),
            same_includes: self.same_includes.merge(new_rules.same_includes),
            multiple_versions: self.multiple_versions.merge(new_rules.multiple_versions),
            version_natb: self.version_natb.merge(new_rules.version_natb),
        }
    }
    pub fn add(&mut self, new_rules: ParseRules) {
        self.display_warns.add(new_rules.display_warns);
        self.same_includes.add(new_rules.same_includes);
        self.multiple_versions.add(new_rules.multiple_versions);
        self.version_natb.add(new_rules.version_natb);
    }

    pub fn display_warns(&self) -> &Rule<bool> {
        &self.display_warns
    }
    pub fn same_includes(&self) -> &Rule<SameIncludes> {
        &self.same_includes
    }
    pub fn multiple_versions(&self) -> &Rule<MultipleVersions> {
        &self.multiple_versions
    }
    pub fn version_not_at_the_beginning(&self) -> &Rule<VersionNotAtTheBeginning> {
        &self.version_natb
    }

    pub fn set_display_warns(&mut self, v: bool) {
        self.display_warns.is_default = false;
        self.display_warns.rule = v;
    }
    pub fn set_same_includes(&mut self, v: SameIncludes) {
        self.same_includes.is_default = false;
        self.same_includes.rule = v;
    }
    pub fn set_multiple_versions(&mut self, v: MultipleVersions) {
        self.multiple_versions.is_default = false;
        self.multiple_versions.rule = v;
    }
    pub fn set_version_not_at_the_beginning(&mut self, v: VersionNotAtTheBeginning) {
        self.version_natb.is_default = false;
        self.version_natb.rule = v;
    }
}

use crate::glsl_expand::id_based_vec::{IDBasedVec, Identifier};

#[derive(Debug, Clone)]
pub struct Mark<T> {
    flag: T,
    start: usize,
    end: usize,
}
impl<T> Mark<T> {
    pub fn new(flag: T, start: usize, end: usize) -> Mark<T> {
        Mark{ flag, start, end }
    }
    pub fn flag(&self)  -> &T { &self.flag }
    pub fn start(&self) -> usize { self.start }
    pub fn end(&self)   -> usize { self.end }
}
impl<T: PartialEq> PartialEq for Mark<T> {
    fn eq(&self, other: &Self) -> bool {
        self.flag == other.flag &&
            self.start == other.start &&
            self.end == other.end
    }
}

#[derive(Debug, Clone)]
pub struct MarkedText<T> {
    text: String,
    marks: IDBasedVec<Mark<T>>,
}
impl<T> MarkedText<T> {
    pub fn new(text: String) -> MarkedText<T> {
        MarkedText {
            text,
            marks: IDBasedVec::new(),
        }
    }

    pub fn set_mark(&mut self, flag: T, start: usize, end: usize) -> Identifier {
        if end > self.text.len() {
            panic!("MarkedText::set_mark - Error: mark cannot point outside of text");
        }
        if end < start {
            panic!("MarkedText::set_mark - Error: mark cannot have negative size");
        }
        self.marks.push(Mark::new(flag, start, end))
    }

    pub fn text(&self) -> &String { &self.text }
    pub fn text_mut(&mut self) -> &mut String { &mut self.text }
    pub fn text_move(self) -> String { self.text }
    pub fn marks(&self) -> &IDBasedVec<Mark<T>> { &self.marks }
    pub fn marks_mut(&mut self) -> &mut IDBasedVec<Mark<T>> { &mut self.marks }

    pub fn get_marks_by<F: Fn(&&Mark<T>) -> bool>(&self, f: F) -> Vec<Identifier>{
        self.marks().find_elements(f)
    }

    pub fn remove_mark(&mut self, mark_id: Identifier, remove_sub_marks: bool) {
        if remove_sub_marks {
            self.remove_sub_marks(mark_id);
        }
        let _ = self.extract_mark(mark_id);
    }

    pub fn remove_sub_marks(&mut self, main_mark_id: Identifier) {
        let main_mark = self.marks().get(main_mark_id);

        if main_mark.is_none() {
            return;
        }
        let main_mark = main_mark.unwrap();

        let elements_to_delete: Vec<Identifier> = self.marks
            .iter()
            .enumerate_slots()
            .filter(|(slot, el)|
                el.start >= main_mark.start &&
                    el.start <= main_mark.end &&
                    el.end >= main_mark.start &&
                    el.end <= main_mark.end &&
                    *slot != main_mark_id.slot())
            .map(|(slot, _)| self.marks.get_id_by_slot(slot).unwrap())
            .collect();
        let _ = self.marks.extract_mul(&elements_to_delete);
    }

    pub fn shift_marks(&mut self, start: usize, shift: isize) {
        for mark in self.marks_mut().iter_mut() {
            if mark.start > start {
                mark.start = add_isize_to_usize(mark.start, shift);
            }
            if mark.end > start {
                mark.end = add_isize_to_usize(mark.end, shift);
            }
        }
    }

    pub fn replace_mark_content(&mut self, id: Identifier, replace_to: MarkedText<T>) {
        let mark = self.marks.get(id);
        if mark.is_none() {
            return;
        }
        let mark = mark.unwrap();
        let mark_text_start = mark.start;
        let mark_text_end = mark.end;
        let shift: isize = replace_to.text().len() as isize - ((mark_text_end - mark_text_start) as isize);

        self.remove_sub_marks(id);
        self.shift_marks(mark_text_start, shift);
        self.text.replace_range(mark_text_start..mark_text_end, replace_to.text());
        self.marks.push_mul(
            replace_to.marks.into_iter()
                .map(|m| Mark::new(m.flag, m.start + mark_text_start, m.end + mark_text_start))
        );
        self.marks.get_mut(id).unwrap().end = add_isize_to_usize(mark_text_end, shift);
        self.marks.get_mut(id).unwrap().start = mark_text_start;
    }

    pub fn delete_mark_content(&mut self, id: Identifier) {
        self.replace_mark_content(id, MarkedText::new("".to_string()));
    }

    pub fn delete_mark_and_content(&mut self, id: Identifier) {
        self.replace_mark_content(id, MarkedText::new("".to_string()));
        self.remove_mark(id, false);
    }

    pub fn extract_mark(&mut self, id: Identifier) -> Option<Mark<T>> {
        self.marks.extract(id)
    }
}
impl<T: PartialEq> MarkedText<T> {

    pub fn get_marks_by_flag(&self, flag: &T) -> Vec<Identifier> {
        self.get_marks_by(move |mark| mark.flag.eq(&flag))
    }
}

pub fn add_isize_to_usize(a: usize, b: isize) -> usize {
    (a as isize + b) as usize
}
use std::borrow::BorrowMut;

#[derive(Clone, Debug)]
pub struct Slot<T> {
    iteration: usize,
    item: Option<T>,
}
impl<T> Slot<T> {
    pub fn new() -> Slot<T> {
        Slot{ iteration: 0, item: None }
    }

    pub fn put(&mut self, item: Option<T>) -> Option<T> {
        self.iteration += 1;
        std::mem::replace(&mut self.item, item)
    }

    pub fn is_some(&self) -> bool {  self.item.is_some()  }
    pub fn is_none(&self) -> bool {  self.item.is_none()  }
}

#[derive(Clone, Copy, Debug)]
pub struct Identifier {
    iteration: usize,
    vec_id: usize
}
impl Identifier {
    pub fn new(iteration: usize, vec_id: usize) -> Identifier {
        Identifier{ iteration, vec_id }
    }
    pub fn iteration(&self) -> usize {
        self.iteration
    }
    pub fn slot(&self) -> usize {
        self.vec_id
    }
}
impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.iteration == other.iteration && self.vec_id == other.vec_id
    }
}

#[derive(Clone, Debug)]
pub struct IDBasedVec<T> {
    data: Vec<Slot<T>>
}
impl<T> IDBasedVec<T> {
    pub fn new() -> IDBasedVec<T> {
        IDBasedVec {
            data: Vec::new()
        }
    }

    pub fn push(&mut self, value: T) -> Identifier {
        let slot_id = self.get_free_slot();
        let _ = self.put(slot_id, Some(value));
        Identifier::new(self.slot_iteration(slot_id).unwrap(), slot_id)
    }
    pub fn push_mul<I: IntoIterator<Item=T>>(&mut self, values: I) -> Vec<Identifier> {
        values
            .into_iter()
            .map(|v| self.push(v) )
            .collect()
    }

    pub fn get(&self, id: Identifier) -> Option<&T> {
        if self.owns_item(id) {
            Some( (&self.data[id.vec_id].item).as_ref().unwrap() )
        } else {
            None
        }
    }
    pub fn get_mul<I: IntoIterator<Item = Identifier>>(&self, ids: I) -> Vec<&T> {
        ids.into_iter()
            .map(|id| self.get(id))
            .filter(|item| item.is_some())
            .map(|item| item.unwrap())
            .collect()
    }

    pub fn get_mut(&mut self, id: Identifier) -> Option<&mut T> {
        if self.owns_item(id) {
            Some( (&mut self.data[id.vec_id].item).as_mut().unwrap() )
        } else {
            None
        }
    }

    pub fn find_elements<F: Fn(&&T) -> bool>(&self, f: F) -> Vec<Identifier> {
        self.iter()
            .enumerate_slots()
            .filter(|(_, item)| f(item))
            .map(|(slot, _)| self.get_id_by_slot(slot).unwrap())
            .collect()
    }
    pub fn current_elements(&self) -> Vec<Identifier> {
        self.find_elements(|_| true)
    }

    pub fn extract(&mut self, id: Identifier) -> Option<T> {
        if self.owns_item(id) {
            self.put(id.vec_id, None)
        } else {
            None
        }
    }
    pub fn extract_mul<'a, I: IntoIterator<Item = &'a Identifier>>(&mut self, ids: I) -> Vec<T> {
        ids.into_iter()
            .map(|id| self.extract(id.clone()))
            .filter(|item| item.is_some())
            .map(|item| item.unwrap())
            .collect()
    }

    pub fn owns_item(&self, id: Identifier) -> bool {
        id.vec_id < self.data.len() &&
        id.iteration == self.data[id.vec_id].iteration &&
        self.data[id.vec_id].is_some()
    }

    pub fn get_id_by_slot(&self, slot: usize) -> Option<Identifier> {
        let get_slot = self.get_slot(slot);
        match get_slot {
            Some(slot_ref) => Some(Identifier::new(slot_ref.iteration, slot)),
            None => None
        }
    }

    pub fn iter(&self) -> VecRefIter<T> {
        self.into_iter()
    }
    pub fn iter_mut(&mut self) -> VecMutRefIter<T> {
        self.into_iter()
    }

    pub fn get_slot(&self, slot: usize) -> Option<&Slot<T>> {
        if slot >= self.data.len() {
            return None;
        }
        Some(&self.data[slot])
    }
    pub fn get_slot_mut(&mut self, slot: usize) -> Option<&mut Slot<T>> {
        if slot >= self.data.len() {
            return None;
        }
        Some(&mut self.data[slot])
    }

    pub fn slots_count(&self) -> usize {
        self.data.len()
    }
    pub fn empty(&self) -> bool {
        for s in &self.data {
            if s.is_some() {
                return false;
            }
        }
        true
    }

    fn put(&mut self, slot: usize, item: Option<T>) -> Option<T> {
        if slot >= self.data.len() {
            return None;
        }
        self.data[slot].put(item)
    }
    fn slot_iteration(&self, slot: usize) -> Option<usize> {
        if slot >= self.data.len() {
            return None;
        }
        Some(self.data[slot].iteration)
    }


    fn get_free_slot(&mut self) -> usize {
        for (i, slot) in self.data.iter().enumerate() {
            if slot.is_none() {
                return i;
            }
        }
        self.data.push(Slot::new());
        self.data.len() - 1
    }
}
impl<T> FromIterator<T> for IDBasedVec<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        let mut res = IDBasedVec::<T>::new();
        for i in iter {
            res.push(i);
        }
        res
    }
}

//Iter of T
pub struct VecIter<T> {
    slot: usize,
    vec: IDBasedVec<T>
}
impl<T> VecIter<T> {
    pub fn enumerate_slots(self) -> VecIterSlotted<T> {
        VecIterSlotted {
            slot: self.slot,
            vec: self.vec,
        }
    }
}
impl<T> Iterator for VecIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.slot..self.vec.data.len() {
            let slot = self.vec.get_slot_mut(i).unwrap();
            if slot.is_some() {
                let item = slot.put(None);
                self.slot = i + 1;
                return item;
            }
        }
        None
    }
}
impl<T> IntoIterator for IDBasedVec<T> {
    type Item = T;
    type IntoIter = VecIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        VecIter {
            slot: 0,
            vec: self,
        }
    }
}

pub struct VecIterSlotted<T> {
    slot: usize,
    vec: IDBasedVec<T>
}
impl<T> Iterator for VecIterSlotted<T> {
    type Item = (usize, T);

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.slot..self.vec.data.len() {
            let slot = self.vec.get_slot_mut(i).unwrap();
            if slot.is_some() {
                let item = slot.put(None);
                self.slot = i + 1;
                return match item {
                    Some(element) => Some((i, element)),
                    None => None,
                };
            }
        }
        None
    }
}

//Iter of &T
pub struct VecRefIter<'a, T> {
    slot: usize,
    vec: &'a IDBasedVec<T>
}
impl<'a, T> VecRefIter<'a, T> {
    pub fn enumerate_slots(self) -> VecRefIterSlotted<'a, T> {
        VecRefIterSlotted {
            slot: self.slot,
            vec: self.vec,
        }
    }
}
impl<'a, T> Iterator for VecRefIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.slot..self.vec.data.len() {
            let slot = self.vec.get_slot(i).unwrap();
            if slot.is_some() {
                let item: &'a T = slot.item.as_ref().unwrap();
                self.slot = i + 1;
                return Some(item);
            }
        }
        None
    }
}
impl<'a, T> IntoIterator for &'a IDBasedVec<T> {
    type Item = &'a T;
    type IntoIter = VecRefIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        VecRefIter {
            slot: 0,
            vec: self,
        }
    }
}

pub struct VecRefIterSlotted<'a, T> {
    slot: usize,
    vec: &'a IDBasedVec<T>
}
impl<'a, T> Iterator for VecRefIterSlotted<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.slot..self.vec.data.len() {
            let slot = self.vec.get_slot(i).unwrap();
            if slot.is_some() {
                let item: &'a T = slot.item.as_ref().unwrap();
                self.slot = i + 1;
                return Some((i, item));
            }
        }
        None
    }
}

//Iter of &mut T
pub struct VecMutRefIter<'a, T> {
    slot: usize,
    data: Vec<Option<&'a mut Slot<T>>>
}
impl<'a, T> VecMutRefIter<'a, T> {
    pub fn enumerate_slots(self) -> VecMutRefIterSlotted<'a, T> {
        VecMutRefIterSlotted {
            slot: self.slot,
            data: self.data,
        }
    }
}
impl<'a, T> Iterator for VecMutRefIter<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.slot..self.data.len() {
            if self.data[i].is_some() {
                let item = std::mem::replace(&mut self.data[i], None).unwrap();
                if item.is_some() {
                    return item.item.borrow_mut().as_mut();
                }
            }
        }
        None
    }
}
impl<'a, T> IntoIterator for &'a mut IDBasedVec<T> {
    type Item = &'a mut T;
    type IntoIter = VecMutRefIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut data: Vec<Option<&'a mut Slot<T>>> = Vec::new();
        unsafe {
            for i in 0..self.data.len() {
                let r = self.data.as_mut_ptr().offset(i as isize);
                data.push( Some(r.as_mut().unwrap()) );
            }
        }
        VecMutRefIter { slot: 0, data }
    }
}

pub struct VecMutRefIterSlotted<'a, T> {
    slot: usize,
    data: Vec<Option<&'a mut Slot<T>>>
}
impl<'a, T> Iterator for VecMutRefIterSlotted<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.slot..self.data.len() {
            if self.data[i].is_some() {
                let item = std::mem::replace(&mut self.data[i], None).unwrap();
                if item.is_some() {
                    return Some((i, item.item.borrow_mut().as_mut().unwrap()));
                }
            }
        }
        None
    }
}
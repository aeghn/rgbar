use std::{sync::atomic::AtomicU32, rc::Rc, cell::RefCell};


#[derive(Clone)]
pub struct Ring<E> 
where E: Clone {
    pub size: usize,
    vec: Rc<RefCell<(usize, Vec<E>)>>
}

impl <E> Ring<E>
where E: Clone {
    pub fn new(size: usize) -> Ring<E> {
        Ring {
            size,
            vec: Default::default()
        }
    }

    fn cursor(&self) -> usize {
        self.vec.borrow().0
    }
    
    pub fn get_all(&self) -> Vec<E> {
        let data = self.vec.borrow();
        let svec = &data.1;
        let cursor = data.0 % self.size;
        let mut vec = svec[cursor..].to_vec();
        
        let mut head = svec[..cursor].to_vec();

        vec.append(&mut head);

        vec
    }

    pub fn add(&self, value: E) {
        let cursor = self.cursor() % self.size;
        if self.vec.borrow().1.len() < self.size {
            self.vec.borrow_mut().1.push(value);
        } else {
            self.vec.borrow_mut().1[cursor] = value;
        }

        self.vec.borrow_mut().0 = cursor + 1
    }
}
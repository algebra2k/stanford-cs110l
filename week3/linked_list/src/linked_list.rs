use std::fmt;
use std::option::Option;

pub struct Iter<'a, T: 'a>(Option<&'a Node<T>>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map(|node| {
            self.0 = node.next.as_ref().map(|node| &**node);
            &node.value
        })
    }
}

pub struct IterMut<'a, T: 'a>(Option<&'a mut Node<T>>);

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        // self.0.take get Option<&'a mut Node<T>> to map &'a mut Node<t>
        // self.0.take can guaraantee that there is only one &mut pointing
        // to value at any time.
        self.0.take().map(|node| {
            self.0 = node.next.as_mut().map(|node| &mut **node);
            &mut node.value
        })
    }
}


impl<T> LinkedList<T> {
    pub fn iter(&self) -> Iter<T> {
        Iter(self.head.as_ref().map(|node| &**node))
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        // 1. as_mut get Option<&mut T> to map
        // 2. *node deref &mut Box get mut Box<Node<T>>
        // 3. **node deref mut Box get mut Node<T>
        // 4. &mut **node get Node<t> mut reference
        IterMut(self.head.as_mut().map(|node| &mut **node))
    }
}


pub struct LinkedListIter<T> {
    cur: Option<Box<Node<T>>>,
}

impl<T> Iterator for LinkedListIter<T>
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.cur.take().map(|mut box_node| {
            self.cur = box_node.next.take();
            box_node.value
        })
    }
}

impl<T> IntoIterator for LinkedList<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = LinkedListIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        LinkedListIter {
            // this will be clone the whole linkedlist
            cur: self.head.clone(),
        }
    }
}

impl<'a, T> IntoIterator for &'a LinkedList<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        Iter(self.head.as_ref().map(|node| &**node))
    }
}

impl<'a, T> IntoIterator for &'a mut LinkedList<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.head.as_mut().map(|node| &mut **node))
    }
}

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {
            value: value,
            next: next,
        }
    }
}

impl<T> Clone for Node<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        println!("clone");
        Node {
            value: self.value.clone(),
            next: self.next.clone(),
        }
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {
            head: None,
            size: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }

    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}

impl<T> Clone for LinkedList<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut new_list = LinkedList::new();
        let current: &Option<Box<Node<T>>> = &self.head;
        loop {
            match current {
                Some(node) => {
                    let new_node: Box<Node<T>> =
                        Box::new(Node::new(node.value.clone(), new_list.head.take()));
                    new_list.head = Some(new_node);
                    new_list.size += 1;
                }
                None => break,
            }
        }

        new_list
    }
}

impl<T> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get_size() == other.get_size()
    }
}


impl<T> fmt::Display for LinkedList<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                }
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

use std::fmt;
use std::option::Option;

pub struct LinkedListIntoIter<T> {
    cur: Option<Box<Node<T>>>,
}

impl<T> Iterator for LinkedListIntoIter<T>
where
    T: Clone,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur.is_none() {
            None
        } else {
            // let next_node: Box<Node<T>> = self.cur.as_ref().unwrap().next.as_ref().unwrap().clone();
            // self.cur = Some(next_node);
            // i didn't get it , i was just in awe.
            let val = self.cur.as_ref().unwrap().value.clone();
            let cur_node = self.cur.take();
            let next_node = cur_node.unwrap().next.take();
            self.cur = next_node;
            Some(val)
        }
    }
}

impl<T> IntoIterator for LinkedList<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = LinkedListIntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        LinkedListIntoIter {
            // this will be clone the whole linkedlist
            cur: self.head.clone(),
        }
    }
}

pub struct LinkedListIterator<'a, T: 'a> {
    cur: &'a Option<Box<Node<T>>>,
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

impl<'a, T> Iterator for LinkedListIterator<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        match self.cur {
            None => None,
            Some(node) => {
                self.cur = &node.next;
                Some(&node.value)
            }
        }
    }
}

impl<'a, T> IntoIterator for &'a LinkedList<T> {
    type Item = &'a T;
    type IntoIter = LinkedListIterator<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        LinkedListIterator { cur: &self.head }
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

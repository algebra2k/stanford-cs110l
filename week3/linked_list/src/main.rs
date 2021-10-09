use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<u32> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..12 {
        list.push_front(i);
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list.to_string()); // ToString impl for anything impl Display

    // If you implement iterator trait:
    for val in &list {
        println!("{}", val);
    }

    for val in list {
        println!("{}", val);
    }

    let mut str_list: LinkedList<String> = LinkedList::new();
    for i in 0..10 {
        str_list.push_front(format!("str-{}", i + 1));
    }

    for val in &str_list {
        println!("{}", val);
    }
}

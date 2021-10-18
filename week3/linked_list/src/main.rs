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
    //
    print!("use &iter for list: ");
    for val in &list {
        print!("{} ", val);
    }
    println!();

    print!("use &iter for list again: ");
    for val in list.iter() {
        print!("{} ", val);
    }
    println!();

    print!("use iter_mut for list (value + 10): ");
    for val in list.iter_mut() {
        *val += 10
    }
    for val in list.iter() {
        print!("{} ", val);
    }
    println!();

    print!("use iter_mut for list (value + 10) again: ");
    for val in &mut list {
        *val += 10
    }
    for val in list.iter() {
        print!("{} ", val);
    }
    println!();

    // move here
    print!("use iter for list: ");
    for val in list {
        print!("{} ", val);
    }
    println!();

    //let mut str_list: LinkedList<String> = LinkedList::new();
    //for i in 0..10 {
    //    str_list.push_front(format!("str-{}", i + 1));
    //}

    //for val in &str_list {
    //    println!("{}", val);
    //}
}

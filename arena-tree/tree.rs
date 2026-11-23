use std::{cell::RefCell, rc::{Rc, Weak}};

// Version 1: arena + indices. No Rc, no RefCell, no lifetimes to fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(usize);

struct Node<T> { value: T, parent: Option<NodeId>, children: Vec<NodeId> }

pub struct Tree<T> { nodes: Vec<Node<T>> }

impl<T> Tree<T> {
    pub fn new(root: T) -> Self {
        Tree { nodes: vec![Node { value: root, parent: None, children: vec![] }] }
    }
    pub fn root(&self) -> NodeId { NodeId(0) }
    pub fn add_child(&mut self, parent: NodeId, value: T) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node { value, parent: Some(parent), children: vec![] });
        self.nodes[parent.0].children.push(id);
        id
    }
    pub fn value(&self, id: NodeId) -> &T { &self.nodes[id.0].value }
    pub fn parent(&self, id: NodeId) -> Option<NodeId> { self.nodes[id.0].parent }
    pub fn depth(&self, mut id: NodeId) -> usize {
        let mut d = 0;
        while let Some(p) = self.parent(id) { id = p; d += 1; }
        d
    }
}

// Version 2: the Rc/Weak/RefCell shape from the Book. Compare how much ceremony it needs.
pub struct RcNode {
    pub value: i32,
    pub parent: RefCell<Weak<RcNode>>,      // Weak so parent <-> child is not a reference cycle
    pub children: RefCell<Vec<Rc<RcNode>>>,
}

fn main() {
    let mut t = Tree::new("root");
    let a = t.add_child(t.root(), "a");
    let b = t.add_child(a, "b");
    println!("{} depth {}", t.value(b), t.depth(b));

    let leaf = Rc::new(RcNode { value: 3, parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) });
    let branch = Rc::new(RcNode { value: 5, parent: RefCell::new(Weak::new()), children: RefCell::new(vec![Rc::clone(&leaf)]) });
    *leaf.parent.borrow_mut() = Rc::downgrade(&branch);
    println!("leaf parent = {:?}", leaf.parent.borrow().upgrade().map(|p| p.value));
    println!("strong={} weak={}", Rc::strong_count(&branch), Rc::weak_count(&branch));
}

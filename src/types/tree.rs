pub struct Node<T> {
    children: Vec<usize>,

    pub data: T,
}

impl<T> Node<T> {
    pub fn new(data: T) -> Self {
        Self {
            children: vec![],
            data: data,
        }
    }

    pub fn add_child(&mut self, child: usize) {
        //let next_index = self.children.len();
        self.children.push(child);
    }

    pub fn get_children(&self) -> &Vec<usize> {
        &self.children
    }
}

pub struct Tree<T> {
    pub nodes: Vec<Node<T>>,
}

impl<T> Tree<T> {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
        }
    }

    pub fn get(&self, index: usize) -> Option<&Node<T>> {
        self.nodes.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Node<T>> {
        self.nodes.get_mut(index)
    }

    pub fn insert_root(&mut self, data: T) -> usize {
        let next_idx = self.nodes.len();
        self.nodes.push(Node::new(data));
        next_idx
    }

    pub fn insert(&mut self, data: T, parent_index: usize) -> usize {
        let next_idx = self.nodes.len();

        let parent = self.get_mut(parent_index).unwrap();
        parent.add_child(next_idx);
        self.nodes.push(Node::new(data));

        next_idx
    }
}


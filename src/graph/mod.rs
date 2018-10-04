use self::vertex::adj::Edge;
use self::vertex::Vertex;
use id::{Id, IdGen};
use rpds::Vector;
use std::fmt::{Debug, Error, Formatter};
use std::iter::{FilterMap, IntoIterator};
use std::ops::{Index, IndexMut};

pub mod vertex;

type GraphInternal<V, E> = Vector<Option<Vertex<V, E>>>;

pub struct Graph<V, E> {
    guts: GraphInternal<V, E>,
    idgen: IdGen,
}

// Derive only implements for <V: Clone, E: Clone> because of rust#26925
impl<V, E> Clone for Graph<V, E> {
    fn clone(&self) -> Self {
        Self {
            guts: self.guts.clone(),
            idgen: self.idgen.clone(),
        }
    }
}

impl<V: Debug, E: Debug> Debug for Graph<V, E> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "Graph (gen {}) {{", self.idgen.get_gen())?;
        let mut any_vertices = false;

        for v in self {
            write!(f, "\n\t{:?}", v)?;
            any_vertices = true;
        }

        write!(f, "{}}}\n", if any_vertices { "\n" } else { "" })?;
        Result::Ok(())
    }
}

// helpers
impl<V, E> Graph<V, E> {
    fn find_empty(&self) -> Option<usize> {
        for (i, v) in self.guts.iter().enumerate() {
            match v {
                None => return Some(i),
                _ => (),
            }
        }
        None
    }

    fn get(&self, id: &Id) -> Option<&Vertex<V, E>> {
        match self.guts.get(id.into()) {
            Some(Some(vertex)) if vertex.same_id(id) => Some(vertex),
            _ => None,
        }
    }
}

impl<V, E> Graph<V, E> {
    pub fn new() -> Self {
        Self {
            guts: GraphInternal::new(),
            idgen: IdGen::new(),
        }
    }

    pub fn add(&self, vertex: V) -> (Self, Id) {
        let mut result = self.clone();
        let id = result.add_mut(vertex);
        (result, id)
    }

    pub fn get_data(&self, id: &Id) -> Option<&V> {
        self.get(id).map(|v| v.get_data())
    }

    pub fn get_edge(&self, source: &Id, sink: &Id) -> Option<&E> {
        self.get(source).and_then(|v| v.get_cost(sink))
    }

    pub fn has_vertex(&self, id: &Id) -> bool {
        self.get(id).is_some()
    }

    pub fn add_mut(&mut self, vertex: V) -> Id {
        match self.find_empty() {
            Some(index) => {
                let id = self.idgen.create_id(index);
                self.guts.set_mut(index, Some(Vertex::from(id, vertex)));
                id
            }
            None => {
                let id = self.idgen.create_id(self.guts.len());
                self.guts.push_back_mut(Some(Vertex::from(id, vertex)));
                id
            }
        }
    }

    pub fn ids(&self) -> Vec<Id> {
        self.guts
            .iter()
            .filter_map(|v_opt| match v_opt {
                Some(v) => Some(*v.get_id()),
                None => None,
            }).collect()
    }

    pub fn vertices_with_edge_to(&self, sink: &Id) -> Vec<Id> {
        self.guts
            .iter()
            .filter_map(|v_opt| match v_opt {
                Some(v) if v.is_connected(sink) => Some(*v.get_id()),
                _ => None,
            }).collect()
    }

    pub fn neighbors(&self, source: &Id) -> Option<impl Iterator<Item = &Edge<E>>> {
        self.get(source).map(|v| v.into_iter())
    }
}

impl<V: Clone, E> Graph<V, E> {
    pub fn get_data_mut(&mut self, id: &Id) -> Option<&mut V> {
        self.get_mut(id).map(|v| v.get_data_mut())
    }

    pub fn connect(&self, source: &Id, sink: &Id, edge: E) -> Self {
        let mut result = self.clone();
        result.connect_mut(source, sink, edge);
        result
    }

    pub fn try_connect(&self, source: &Id, sink: &Id, edge: E) -> Option<Self> {
        let mut result = self.clone();
        if result.connect_mut(source, sink, edge) {
            Some(result)
        } else {
            None
        }
    }

    pub fn connect_mut(&mut self, source: &Id, sink: &Id, edge: E) -> bool {
        if let Some(v) = self.get_mut(source) {
            v.connect_to(sink, edge);
            true
        } else {
            false
        }
    }

    fn get_mut(&mut self, id: &Id) -> Option<&mut Vertex<V, E>> {
        match self.guts.get_mut(id.into()) {
            Some(Some(vertex)) if vertex.same_id(id) => Some(vertex),
            _ => None,
        }
    }
}

impl<V: Clone, E: Clone> Graph<V, E> {
    pub fn get_edge_mut(&mut self, source: &Id, sink: &Id) -> Option<&mut E> {
        self.get_mut(source).and_then(|v| v.get_cost_mut(sink))
    }

    pub fn remove(&self, id: &Id) -> Self {
        let mut result = self.clone();
        result.remove_mut_no_inc(id);
        result
    }

    pub fn try_remove(&self, id: &Id) -> Option<Self> {
        let mut result = self.clone();
        if result.remove_mut_no_inc(id) {
            Some(result)
        } else {
            None
        }
    }

    pub fn remove_all<'a, I: IntoIterator<Item = &'a Id>>(&self, iter: I) -> Self {
        let mut result = self.clone();
        result.remove_all_mut_no_inc(iter);
        result
    }

    pub fn try_remove_all<'a, I: IntoIterator<Item = &'a Id>>(&self, iter: I) -> Option<Self> {
        let mut result = self.clone();
        if result.remove_all_mut_no_inc(iter) {
            Some(result)
        } else {
            None
        }
    }

    pub fn disconnect(&self, source: &Id, sink: &Id) -> Self {
        let mut result = self.clone();
        result.disconnect_mut(source, sink);
        result
    }

    pub fn try_disconnect(&self, source: &Id, sink: &Id) -> Option<Self> {
        let mut result = self.clone();
        if result.disconnect_mut(source, sink) {
            Some(result)
        } else {
            None
        }
    }

    pub fn remove_mut(&mut self, id: &Id) -> bool {
        let changed = self.remove_mut_no_inc(id);
        if changed {
            self.idgen.next_gen();
        };
        changed
    }

    pub fn remove_all_mut<'a, I: IntoIterator<Item = &'a Id>>(&mut self, iter: I) -> bool {
        let changed = self.remove_all_mut_no_inc(iter);
        if changed {
            self.idgen.next_gen();
        };
        changed
    }

    fn remove_all_mut_no_inc<'a, I: IntoIterator<Item = &'a Id>>(&mut self, iter: I) -> bool {
        let changed = iter
            .into_iter()
            .fold(false, |changed, id| self.remove_mut_no_inc(id) || changed);
        changed
    }

    fn remove_mut_no_inc(&mut self, id: &Id) -> bool {
        if self.has_vertex(id) {
            self.guts.set_mut(id.into(), None);
            self.disconnect_all_inc_mut(id);
            true
        } else {
            false
        }
    }

    pub fn disconnect_mut(&mut self, source: &Id, sink: &Id) -> bool {
        self.get_mut(source).map_or(false, |v| v.disconnect(sink))
    }

    fn disconnect_all_inc_mut(&mut self, sink: &Id) -> bool {
        self.vertices_with_edge_to(sink)
            .iter()
            .fold(false, |init, source| {
                self.disconnect_mut(source, sink) || init
            })
    }
}

impl<V, E> Index<Id> for Graph<V, E> {
    type Output = Vertex<V, E>;

    fn index(&self, id: Id) -> &Vertex<V, E> {
        self.get(&id).unwrap()
    }
}

impl<V: Clone, E> IndexMut<Id> for Graph<V, E> {
    fn index_mut(&mut self, id: Id) -> &mut Vertex<V, E> {
        self.get_mut(&id).unwrap()
    }
}

impl<'a, V, E> IntoIterator for &'a Graph<V, E> {
    type Item = &'a Vertex<V, E>;
    type IntoIter = FilterMap<
        <&'a GraphInternal<V, E> as IntoIterator>::IntoIter,
        fn(&'a Option<Vertex<V, E>>) -> Option<&'a Vertex<V, E>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.guts.iter().filter_map(|v_opt| v_opt.as_ref())
    }
}

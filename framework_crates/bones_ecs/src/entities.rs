//! [`Entity`] implementation, storage, and interation.

use std::{marker::PhantomData, rc::Rc};

use crate::prelude::*;

/// An entity index.
///
/// They are created using the `Entities` struct. They are used as indices with `Components`
/// structs.
///
/// Entities are conceptual "things" which possess attributes (Components). As an exemple, a Car
/// (Entity) has a Color (Component), a Position (Component) and a Speed (Component).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(HasSchema, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct Entity(u32, u32);
impl Entity {
    /// An invalid entity, useful for placeholder entity values.
    const INVALID: Entity = Entity(u32::MAX, u32::MAX);

    /// Creates a new `Entity` from the provided index and generation.
    ///
    /// > ⚠️ **Warning:** It is not generally recommended to manually create [`Entity`]s unless you
    /// > know exactly what you are doing. This can be useful in certain advanced or unusual
    /// > use-cases, but usually you should use [`Entities::create()`] to spawn entities.
    pub fn new(index: u32, generation: u32) -> Entity {
        Entity(index, generation)
    }

    /// Returns the index of this `Entity`.
    ///
    /// In most cases, you do not want to use this directly.
    /// However, it can be useful to create caches to improve performances.
    pub fn index(&self) -> u32 {
        self.0
    }

    /// Returns the generation of this `Entity`.
    ///
    ///
    /// In most cases, you do not want to use this directly.
    /// However, it can be useful to create caches to improve performances.
    pub fn generation(&self) -> u32 {
        self.1
    }
}
impl Default for Entity {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Holds a list of alive entities.
///
/// It also holds a list of entities that were recently killed, which allows to remove components of
/// deleted entities at the end of a game frame.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, HasSchema)]
pub struct Entities {
    /// Bitset containing all living entities
    alive: BitSetVec,
    generation: Vec<u32>,
    killed: Vec<Entity>,
    next_id: usize,
    /// helps to know if we should directly append after next_id or if we should look through the
    /// bitset.
    has_deleted: bool,
}
impl std::fmt::Debug for Entities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entities").finish_non_exhaustive()
    }
}

impl Default for Entities {
    fn default() -> Self {
        Self {
            alive: create_bitset(),
            generation: vec![0u32; BITSET_SIZE],
            killed: vec![],
            next_id: 0,
            has_deleted: false,
        }
    }
}

/// A type representing a component-joining entity query.
pub trait QueryItem {
    /// The type of iterator this query item creates
    type Iter: Iterator;
    /// Modify the iteration bitset
    fn apply_bitset(&self, bitset: &mut BitSetVec);
    /// Return the item that matches the query within the given bitset if there is exactly one
    /// entity that matches this query item.
    fn get_single_with_bitset(
        self,
        bitset: Rc<BitSetVec>,
    ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError>;
    /// Return an iterator over the provided bitset.
    fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter;
}

/// An error that may occur when querying for a single entity. For example, via
/// [`Entities::get_single_with`], or more directly with
/// [`ComponentStore::get_single_with_bitset`] or
/// [`ComponentStore::get_single_mut_with_bitset`].
#[derive(Debug, PartialEq, Eq)]
pub enum QuerySingleError {
    /// No entity matches the query.
    NoEntities,
    /// More than one entity matches the query.
    MultipleEntities,
}

/// Wrapper for the [`Comp`] [`SystemParam`] used as [`QueryItem`] to iterate
/// over entities optionally retrieving components from [`ComponentStore`].
/// Entities iterated over will not be filtered by [`OptionalQueryItem`].
///
/// See [`Optional`] helper func for constructing `OptionalQueryItem` and usage.
pub struct OptionalQueryItem<'a, T: HasSchema, S>(pub &'a S, pub PhantomData<&'a T>);

/// Wrapper for the [`CompMut`] [`SystemParam`] used as [`QueryItem`] to iterate
/// over entities optionally and mutably retrieving components from [`ComponentStore`].
/// Entities iterated over will not be filtered by [`OptionalQueryItemMut`].
///
/// See [`OptionalMut`] helper func for constructing `OptionalQueryItemMut` and usage
pub struct OptionalQueryItemMut<'a, T: HasSchema, S>(pub &'a mut S, pub PhantomData<&'a T>);

/// Helper func to construct a [`OptionalQueryItem`] wrapping a [`Comp`] [`SystemParam`].
/// Used to iterate over enities optionally retrieving components from [`ComponentStore`].
/// Entities iterated over will not be filtered by this `QueryItem`.
///
/// This example filters entities by `compC`, optionally retrieves `compA` as mutable, and
/// `compB` as immutable. ([`OptionalMut`] is used for mutation).
///
/// `entities.iter_with(&mut OptionalMut(&mut compA), &Optional(&compB), &compC)`
///
/// This will implement [`QueryItem`] as long as generic type implements [`std::ops::Deref`] for
/// [`ComponentStore`], such as [`Comp`] and [`CompMut`].
#[allow(non_snake_case)]
pub fn Optional<'a, T: HasSchema, C, S>(component_ref: &'a S) -> OptionalQueryItem<'a, T, S>
where
    C: ComponentIterBitset<'a, T> + 'a,
    S: std::ops::Deref<Target = C> + 'a,
{
    OptionalQueryItem(component_ref, PhantomData)
}

/// Helper func to construct a [`OptionalQueryItemMut`] wrapping a [`CompMut`] [`SystemParam`].
/// Used to iterate over enities optionally and mutably retrieving components from [`ComponentStore`].
/// Entities iterated over will not be filtered by this `QueryItem`.
///
/// This example filters entities by `compC`, optionally retrieves `compA` as mutable, and
/// `compB` as immutable.
///
/// `entities.iter_with(&mut OptionalMut(&mut compA), &Optional(&compB), &compC)`
///
/// This will implement [`QueryItem`] as long as generic type implements [`std::ops::DerefMut`] for
/// [`ComponentStore`], such as [`CompMut`].
#[allow(non_snake_case)]
pub fn OptionalMut<'a, T: HasSchema, C, S>(
    component_ref: &'a mut S,
) -> OptionalQueryItemMut<'a, T, S>
where
    C: ComponentIterBitset<'a, T> + 'a,
    S: std::ops::DerefMut<Target = C> + 'a,
{
    OptionalQueryItemMut(component_ref, PhantomData)
}

impl<'a> QueryItem for &'a Ref<'a, UntypedComponentStore> {
    type Iter = UntypedComponentBitsetIterator<'a>;

    fn apply_bitset(&self, bitset: &mut BitSetVec) {
        bitset.bit_and(self.bitset());
    }

    fn get_single_with_bitset(
        self,
        bitset: Rc<BitSetVec>,
    ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError> {
        UntypedComponentStore::get_single_with_bitset(self, bitset)
    }

    fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter {
        UntypedComponentStore::iter_with_bitset(self, bitset)
    }
}

impl<'a, 'q, T: HasSchema> QueryItem for &'a Comp<'q, T> {
    type Iter = ComponentBitsetIterator<'a, T>;

    fn apply_bitset(&self, bitset: &mut BitSetVec) {
        bitset.bit_and(self.bitset());
    }

    fn get_single_with_bitset(
        self,
        bitset: Rc<BitSetVec>,
    ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError> {
        ComponentStore::get_single_with_bitset(&**self, bitset)
    }

    fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter {
        ComponentStore::iter_with_bitset(&**self, bitset)
    }
}

impl<'a, 'q, T: HasSchema> QueryItem for &'a CompMut<'q, T> {
    type Iter = ComponentBitsetIterator<'a, T>;

    fn apply_bitset(&self, bitset: &mut BitSetVec) {
        bitset.bit_and(self.bitset());
    }

    fn get_single_with_bitset(
        self,
        bitset: Rc<BitSetVec>,
    ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError> {
        ComponentStore::get_single_with_bitset(&**self, bitset)
    }

    fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter {
        ComponentStore::iter_with_bitset(&**self, bitset)
    }
}

impl<'a, 'q, T: HasSchema> QueryItem for &'a mut CompMut<'q, T> {
    type Iter = ComponentBitsetIteratorMut<'a, T>;

    fn apply_bitset(&self, bitset: &mut BitSetVec) {
        bitset.bit_and(self.bitset());
    }

    fn get_single_with_bitset(
        self,
        bitset: Rc<BitSetVec>,
    ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError> {
        ComponentStore::get_single_with_bitset_mut(self, bitset)
    }

    fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter {
        ComponentStore::iter_mut_with_bitset(self, bitset)
    }
}

/// Immutably iterate over optional component with syntax: `&Optional(&Comp<T>)` / `&Optional(&CompMut<T>)`.
/// (For mutable optional iteration we require `&mut OptionalMut(&mut CompMut<T>)`)
impl<'a, T: HasSchema, S, C> QueryItem for &'a OptionalQueryItem<'a, T, S>
where
    C: ComponentIterBitset<'a, T> + 'a,
    S: std::ops::Deref<Target = C> + 'a,
{
    type Iter = ComponentBitsetOptionalIterator<'a, T>;

    fn apply_bitset(&self, _bitset: &mut BitSetVec) {}

    fn get_single_with_bitset(
        self,
        bitset: Rc<BitSetVec>,
    ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError> {
        match self.0.get_single_with_bitset(bitset) {
            Ok(single) => Ok(Some(single)),
            Err(QuerySingleError::NoEntities) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter {
        self.0.iter_with_bitset_optional(bitset)
    }
}

/// Mutably iterate over optional component with syntax: `&mut OptionalMut(&mut RefMut<ComponentStore<T>>)`
impl<'a, T: HasSchema, S, C> QueryItem for &'a mut OptionalQueryItemMut<'a, T, S>
where
    C: ComponentIterBitset<'a, T> + 'a,
    S: std::ops::DerefMut<Target = C> + 'a,
{
    type Iter = ComponentBitsetOptionalIteratorMut<'a, T>;

    fn apply_bitset(&self, _bitset: &mut BitSetVec) {}

    fn get_single_with_bitset(
        self,
        bitset: Rc<BitSetVec>,
    ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError> {
        match self.0.get_single_mut_with_bitset(bitset) {
            Ok(x) => Ok(Some(x)),
            Err(QuerySingleError::NoEntities) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter {
        self.0.iter_mut_with_bitset_optional(bitset)
    }
}

#[doc(hidden)]
pub struct MultiQueryIter<T> {
    data: T,
}

macro_rules! impl_query {
    ( $( $args:ident, )* ) => {
        impl<
            'q,
            $(
                $args: Iterator,
            )*
        >
        Iterator for MultiQueryIter<($($args,)*)> {
            type Item = (
                $(
                    $args::Item,
                )*
            );

            #[allow(non_snake_case)]
            fn next(&mut self) -> Option<Self::Item> {
                let (
                    $(
                        $args,
                    )*
                ) = &mut self.data;

                match (
                    $(
                        $args.next(),
                    )*
                ) {
                    (
                        $(
                            Some($args),
                        )*
                    ) => Some((
                        $(
                            $args,
                        )*
                    )),
                    _ => None
                }
            }
        }

        impl<
            $(
                $args: QueryItem,
            )*
        > QueryItem for (
            $(
                $args,
            )*
        ) {
            type Iter = MultiQueryIter< (
                $(
                    <$args as QueryItem>::Iter,
                )*
            )>;

            #[allow(non_snake_case)]
            fn apply_bitset(&self, bitset: &mut BitSetVec) {
                let (
                    $(
                        $args,
                    )*
                ) = self;
                $(
                    $args.apply_bitset(bitset);
                )*
            }

            #[allow(non_snake_case)]
            fn get_single_with_bitset(
                self,
                bitset: Rc<BitSetVec>,
            ) -> Result<<Self::Iter as Iterator>::Item, QuerySingleError> {
                let (
                    $(
                        $args,
                    )*
                ) = self;
                let mut query = MultiQueryIter {
                    data: (
                        $(
                            $args.iter_with_bitset(bitset.clone()),
                        )*
                    )
                };
                let first = query.next();
                let has_second = query.next().is_some();
                match (first, has_second) {
                    (None, _) => Err(QuerySingleError::NoEntities),
                    (Some(items), false) => Ok(items),
                    (Some(_), true) => Err(QuerySingleError::MultipleEntities),
                }
            }

            #[allow(non_snake_case)]
            fn iter_with_bitset(self, bitset: Rc<BitSetVec>) -> Self::Iter {
                let (
                    $(
                        $args,
                    )*
                ) = self;
                MultiQueryIter {
                    data: (
                        $(
                            $args.iter_with_bitset(bitset.clone()),
                        )*
                    ),
                }
            }
        }
    };
}

macro_rules! impl_queries {
    // base case
    () => {};
    (
        $head:ident,
        $(
            $tail:ident,
        )*
    ) => {
        // recursive call
        impl_query!($head, $( $tail, )* );
        impl_queries!($( $tail, )* );
    }
}

impl_queries!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);

/// Iterator over entities returned by [`Entities::iter_with`].
pub struct EntitiesIterWith<'e, I> {
    current_id: usize,
    next_id: usize,
    bitset: Rc<BitSetVec>,
    generations: &'e Vec<u32>,
    query: I,
}

impl<'a, I: Iterator> Iterator for EntitiesIterWith<'a, I> {
    type Item = (Entity, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.bitset.bit_test(self.current_id) && self.current_id < self.next_id {
            self.current_id += 1;
        }

        if self.current_id >= self.next_id {
            return None;
        }

        let entity = Entity::new(self.current_id as u32, self.generations[self.current_id]);

        self.current_id += 1;
        self.query.next().map(|item| (entity, item))
    }
}

impl Entities {
    /// Get a single entity and components in the given query if there is exactly one entity
    /// matching the query.
    ///
    /// # Panics
    ///
    /// This method panics if the number of matching entities is not *exactly one*.
    pub fn single_with<Q: QueryItem>(
        &self,
        query: Q,
    ) -> (Entity, <<Q as QueryItem>::Iter as Iterator>::Item) {
        self.get_single_with(query).unwrap()
    }

    /// Get a single entity and components in the given query if there is exactly one entity
    /// matching the query.
    pub fn get_single_with<Q: QueryItem>(
        &self,
        query: Q,
    ) -> Result<(Entity, <<Q as QueryItem>::Iter as Iterator>::Item), QuerySingleError> {
        let mut bitset = self.bitset().clone();
        query.apply_bitset(&mut bitset);

        let entity = {
            let mut ids = (0..self.next_id).filter(|&i| bitset.bit_test(i));
            let id = ids.next().ok_or(QuerySingleError::NoEntities)?;
            if ids.next().is_some() {
                return Err(QuerySingleError::MultipleEntities);
            }
            Entity::new(id as u32, self.generation[id])
        };

        let bitset = Rc::new(bitset);

        query
            .get_single_with_bitset(bitset)
            .map(|item| (entity, item))
    }

    /// Get the first entity in the given bitset.
    ///
    /// # Panics
    ///
    /// This method panics if there are no entities in the bitset.
    pub fn first_with_bitset(&self, bitset: &BitSetVec) -> Entity {
        self.get_first_with_bitset(bitset).unwrap()
    }

    /// Get the first entity in the given bitset.
    pub fn get_first_with_bitset(&self, bitset: &BitSetVec) -> Option<Entity> {
        self.iter_with_bitset(bitset).next()
    }

    /// Get the first entity and components in the given query.
    ///
    /// # Panics
    ///
    /// This method panics if there are no entities that match the query.
    pub fn first_with<Q: QueryItem>(
        &self,
        query: Q,
    ) -> (Entity, <<Q as QueryItem>::Iter as Iterator>::Item) {
        self.get_first_with(query).unwrap()
    }

    /// Get the first entity and components in the given query.
    pub fn get_first_with<Q: QueryItem>(
        &self,
        query: Q,
    ) -> Option<(Entity, <<Q as QueryItem>::Iter as Iterator>::Item)> {
        self.iter_with(query).next()
    }

    /// Iterates over entities using the provided bitset.
    pub fn iter_with_bitset<'a>(&'a self, bitset: &'a BitSetVec) -> EntityIterator {
        EntityIterator {
            current_id: 0,
            next_id: self.next_id,
            entities: &self.alive,
            generations: &self.generation,
            bitset,
        }
    }

    /// Iterate over the entities and components in the given query.
    ///
    /// The [`QueryItem`] trait is automatically implemented for references to [`Comp`] and
    /// [`CompMut`] and for tuples of up to 26 items, so you can join over your mutable or immutable
    /// component borrows in your systems.
    ///
    /// You can also pass a single component, to iterate only over the components that have alive
    /// entities.
    ///
    /// # Example
    ///
    /// ```
    /// # use bones_ecs::prelude::*;
    /// # #[derive(HasSchema, Clone, Default)]
    /// # #[repr(C)]
    /// # struct Pos { x: f32, y: f32 };
    /// # #[derive(HasSchema, Clone, Default)]
    /// # #[repr(C)]
    /// # struct Vel { x: f32, y: f32 };
    ///
    /// fn my_system(entities: Res<Entities>, mut pos: CompMut<Pos>, vel: Comp<Vel>) {
    ///     for (entity, (pos, vel)) in entities.iter_with((&mut pos, &vel)) {
    ///         pos.x += vel.x;
    ///         pos.y += vel.y;
    ///     }
    /// }
    /// ```
    ///
    /// You may optionally iterate over components with `&Optional(&comp)` or mutably with
    /// `&mut OptionalMut(&mut comp_mut)`. Entities are not filtered by component in [`OptionalQueryItem`].
    /// None is returned for these. If done with single Optional query item, all entities are iterated over.
    ///
    /// Syntax is `&Optional(&comp)`, or `&mut OptionalMut(&mut comp)`. Reference to comp and reference to Optional
    /// is required for now.
    ///
    /// # [`Optional`] Example
    ///
    /// ```
    /// # use bones_ecs::prelude::*;
    /// # #[derive(HasSchema, Clone, Default)]
    /// # #[repr(C)]
    /// # struct Pos { x: f32, y: f32 };
    /// # #[derive(HasSchema, Clone, Default)]
    /// # #[repr(C)]
    /// # struct Vel { x: f32, y: f32 };
    /// # #[derive(HasSchema, Clone, Default)]
    /// # #[repr(C)]
    /// # struct PosMax { x: f32, y: f32 }
    ///
    /// fn my_system(entities: Res<Entities>, mut pos: CompMut<Pos>, vel: Comp<Vel>, pos_max: Comp<PosMax>) {
    ///     for (entity, (pos, vel, pos_max)) in entities.iter_with((&mut pos, &vel, &Optional(&pos_max))) {
    ///         // Update pos from vel on all entities that have pos and vel components
    ///         pos.x += vel.x;
    ///         pos.y += vel.y;
    ///
    ///         // limit pos.x by pos_max.x if entity has PosMax component
    ///         if let Some(pos_max) = pos_max {
    ///             if pos.x > pos_max.x {
    ///                 pos.x = pos_max.x
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn iter_with<Q: QueryItem>(&self, query: Q) -> EntitiesIterWith<<Q as QueryItem>::Iter> {
        let mut bitset = self.bitset().clone();
        query.apply_bitset(&mut bitset);
        let bitset = Rc::new(bitset);

        EntitiesIterWith {
            current_id: 0,
            next_id: self.next_id,
            bitset: bitset.clone(),
            generations: &self.generation,
            query: query.iter_with_bitset(bitset),
        }
    }

    /// Creates a new `Entity` and returns it.
    ///
    /// This function will not reuse the index of an entity that is still in the killed entities.
    pub fn create(&mut self) -> Entity {
        if !self.has_deleted {
            let i = self.next_id;
            if i >= BITSET_SIZE {
                panic!("Exceeded maximum amount of concurrent entities.");
            }
            self.next_id += 1;
            self.alive.bit_set(i);
            Entity::new(i as u32, self.generation[i])
        } else {
            let mut section = 0;
            // Find section where at least one bit isn't set
            while self.alive[section].bit_all() {
                section += 1;
            }
            let mut i = section * (32 * 8);
            while self.alive.bit_test(i) || self.killed.iter().any(|e| e.index() == i as u32) {
                i += 1;
            }
            self.alive.bit_set(i);
            if i >= self.next_id {
                self.next_id = i + 1;
                self.has_deleted = false;
            }
            if i >= BITSET_SIZE {
                panic!("Exceeded maximum amount of concurrent entities.");
            }
            let entity = Entity::new(i as u32, self.generation[i]);

            // Make sure we never return the invalid entity.
            if unlikely(entity == Entity::INVALID) {
                panic!("Ran out of entity IDs");
            }

            entity
        }
    }

    /// Checks if the `Entity` is still alive.
    ///
    /// Returns true if it is alive. Returns false if it has been killed.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive.bit_test(entity.index() as usize)
            && self.generation[entity.index() as usize] == entity.generation()
    }

    /// Kill an entity.
    pub fn kill(&mut self, entity: Entity) {
        if self.alive.bit_test(entity.index() as usize) {
            self.alive.bit_reset(entity.index() as usize);
            self.generation[entity.index() as usize] += 1;
            self.killed.push(entity);
            self.has_deleted = true;
        }
    }

    /// Returns a list of all `Entity`s cloned into a new vec.
    pub fn all_cloned(&self) -> Vec<Entity> {
        self.iter().collect()
    }

    /// Kills all entities.
    pub fn kill_all(&mut self) {
        let entities: Vec<Entity> = self.all_cloned();
        for entity in entities {
            self.kill(entity);
        }
    }

    /// Returns entities in the killed list.
    pub fn killed(&self) -> &Vec<Entity> {
        &self.killed
    }

    /// Clears the killed entity list.
    pub fn clear_killed(&mut self) {
        self.killed.clear();
    }

    /// Returns a bitset where each index where the bit is set to 1 indicates the index of an alive
    /// entity.
    ///
    /// Useful for joining over [`Entity`] and [`ComponentStore<T>`] at the same time.
    pub fn bitset(&self) -> &BitSetVec {
        &self.alive
    }

    /// Iterates over all alive entities.
    pub fn iter(&self) -> EntityIterator {
        EntityIterator {
            current_id: 0,
            next_id: self.next_id,
            entities: self.bitset(),
            generations: &self.generation,
            bitset: self.bitset(),
        }
    }
}

/// Iterator over entities using the provided bitset.
pub struct EntityIterator<'a> {
    pub(crate) current_id: usize,
    pub(crate) next_id: usize,
    pub(crate) entities: &'a BitSetVec,
    pub(crate) generations: &'a Vec<u32>,
    //pub(crate) bitset: &'a BitSetVec,
    pub(crate) bitset: &'a BitSetVec,
}

impl<'a> Iterator for EntityIterator<'a> {
    type Item = Entity;
    fn next(&mut self) -> Option<Self::Item> {
        while !(self.bitset.bit_test(self.current_id) && self.entities.bit_test(self.current_id))
            && self.current_id < self.next_id
        {
            self.current_id += 1;
        }
        let ret = if self.current_id < self.next_id {
            Some(Entity::new(
                self.current_id as u32,
                self.generations[self.current_id],
            ))
        } else {
            None
        };
        self.current_id += 1;
        ret
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use std::{collections::HashSet, rc::Rc};

    use crate::prelude::*;

    #[derive(Debug, Clone, PartialEq, Eq, HasSchema, Default)]
    #[repr(C)]
    struct A(String);

    #[derive(Debug, Clone, PartialEq, Eq, HasSchema, Default)]
    #[repr(C)]
    struct B(String);

    #[test]
    fn query_item__get_single_with_one_required() {
        let mut entities = Entities::default();
        let state = AtomicCell::new(ComponentStore::<A>::default());

        {
            let comp = state.borrow_mut();
            let query = &comp;

            let mut bitset = entities.bitset().clone();
            query.apply_bitset(&mut bitset);

            let maybe_comps = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(maybe_comps, Err(QuerySingleError::NoEntities));
        }

        {
            let mut comp = state.borrow_mut();

            let e = entities.create();
            let a = A("e".to_string());
            comp.insert(e, a.clone());

            let query = &comp;
            let mut bitset = entities.bitset().clone();
            query.apply_bitset(&mut bitset);

            let maybe_comps = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(maybe_comps, Ok(&a));

            entities.kill(e);
        }

        {
            let mut comp = state.borrow_mut();

            let e1 = entities.create();
            comp.insert(e1, A("e1".to_string()));

            let e2 = entities.create();
            comp.insert(e2, A("e2".to_string()));

            let query = &comp;
            let mut bitset = entities.bitset().clone();
            query.apply_bitset(&mut bitset);

            let maybe_comps = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(maybe_comps, Err(QuerySingleError::MultipleEntities));

            entities.kill(e1);
            entities.kill(e2);
        }
    }

    #[test]
    fn query_item__get_single_with_multiple_required() {
        let mut entities = Entities::default();
        let state_a = AtomicCell::new(ComponentStore::<A>::default());
        let state_b = AtomicCell::new(ComponentStore::<B>::default());

        {
            let query = (&state_a.borrow(), &state_b.borrow());
            let mut bitset = entities.bitset().clone();
            query.apply_bitset(&mut bitset);

            let maybe_comps = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(maybe_comps, Err(QuerySingleError::NoEntities));
        }

        {
            let e = entities.create();
            let a = A("e".to_string());
            let b = B("e".to_string());
            state_a.borrow_mut().insert(e, a.clone());
            state_b.borrow_mut().insert(e, b.clone());

            let query = (&state_a.borrow(), &state_b.borrow());
            let mut bitset = entities.bitset().clone();
            query.apply_bitset(&mut bitset);

            let maybe_comps = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(maybe_comps, Ok((&a, &b)));

            entities.kill(e);
        }

        {
            let e1 = entities.create();
            state_a.borrow_mut().insert(e1, A("e1".to_string()));
            state_b.borrow_mut().insert(e1, B("e1".to_string()));

            let e2 = entities.create();
            state_a.borrow_mut().insert(e2, A("e2".to_string()));
            state_b.borrow_mut().insert(e2, B("e2".to_string()));

            let query = (&state_a.borrow(), &state_b.borrow());
            let mut bitset = entities.bitset().clone();
            query.apply_bitset(&mut bitset);

            let maybe_comps = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(maybe_comps, Err(QuerySingleError::MultipleEntities));

            entities.kill(e1);
            entities.kill(e2);
        }
    }

    #[test]
    fn query_item__get_single_with_bitset__uses_bitset() {
        let mut entities = Entities::default();
        let state = AtomicCell::new(ComponentStore::<A>::default());

        let e = entities.create();
        state.borrow_mut().insert(e, A("unexpected".to_string()));

        let query = &state.borrow();
        let bitset = Rc::new({
            let mut bitset = BitSetVec::default();
            bitset.bit_set(99);
            bitset
        });

        let maybe_comp = query.get_single_with_bitset(bitset);

        assert_eq!(maybe_comp, Err(QuerySingleError::NoEntities));
    }

    #[test]
    fn entities__create_kill() {
        let mut entities = Entities::default();
        let e1 = entities.create();
        let e2 = entities.create();
        let e3 = entities.create();
        assert_eq!(e1.index(), 0);
        assert_eq!(e2.index(), 1);
        assert_eq!(e3.index(), 2);
        assert_eq!(e1.generation(), 0);
        assert!(entities.is_alive(e1));
        assert!(entities.is_alive(e2));
        assert!(entities.is_alive(e3));
        entities.kill(e1);
        assert!(!entities.is_alive(e1));
        assert!(entities.is_alive(e2));
        assert!(entities.is_alive(e3));
        let e4 = entities.create();
        assert!(!entities.is_alive(e1));
        assert!(entities.is_alive(e2));
        assert!(entities.is_alive(e3));
        assert!(entities.is_alive(e4));

        assert_eq!(*entities.killed(), vec![e1]);
        entities.clear_killed();
        assert_eq!(*entities.killed(), vec![]);
    }

    #[test]
    fn entities__interleaved_create_kill() {
        let mut entities = Entities::default();

        let e1 = entities.create();
        assert_eq!(e1.index(), 0);
        let e2 = entities.create();
        assert_eq!(e2.index(), 1);
        entities.kill(e1);
        entities.kill(e2);
        assert!(!entities.is_alive(e1));
        assert!(!entities.is_alive(e2));

        let e3 = entities.create();
        assert_eq!(e3.index(), 2);
        let e4 = entities.create();
        assert_eq!(e4.index(), 3);
        entities.kill(e3);
        entities.kill(e4);
        assert!(!entities.is_alive(e3));
        assert!(!entities.is_alive(e4));
    }

    #[test]
    /// Exercise basic operations on entities to increase code coverage
    fn entities__clone_debug_hash() {
        let mut entities = Entities::default();
        let e1 = entities.create();
        // Clone
        #[allow(clippy::clone_on_copy)]
        let _ = e1.clone();
        // Debug
        println!("{e1:?}");
        // Hash
        let mut h = HashSet::new();
        h.insert(e1);
    }

    /// Test to cover the code where an entity is allocated in the next free section.
    ///
    /// Exercises a code path not tested according to code coverage.
    #[test]
    fn entities__force_generate_next_section() {
        let mut entities = Entities::default();
        // Create enough entities to fil up the first section of the bitset
        for _ in 0..256 {
            entities.create();
        }
        // Create another entity ( this will be the second section)
        let e1 = entities.create();
        // Kill the entity ( now we will have a deleted entity, but not in the first section )
        entities.kill(e1);
        // Create a new entity
        entities.create();
    }

    #[cfg(not(miri))] // This test is very slow on miri and not critical to test for.
    #[test]
    #[should_panic(expected = "Exceeded maximum amount")]
    fn entities__force_max_entity_panic() {
        let mut entities = Entities::default();
        for _ in 0..(BITSET_SIZE + 1) {
            entities.create();
        }
    }

    #[cfg(not(miri))] // This test is very slow on miri and not critical to test for.
    #[test]
    #[should_panic(expected = "Exceeded maximum amount")]
    fn entities__force_max_entity_panic2() {
        let mut entities = Entities::default();
        let mut e = None;
        for _ in 0..BITSET_SIZE {
            e = Some(entities.create());
        }
        let e = e.unwrap();
        entities.kill(e);
        entities.create();
        entities.create();
    }

    #[test]
    fn entities__iter_with_empty_bitset() {
        let mut entities = Entities::default();

        // Create a couple entities
        entities.create();
        entities.create();

        // Join with an empty bitset
        let bitset = BitSetVec::default();
        assert_eq!(entities.iter_with_bitset(&bitset).count(), 0);
    }

    #[test]
    fn entities__get_single__with_one_required__ok() {
        let mut entities = Entities::default();
        (0..3).map(|_| entities.create()).count();
        let e = entities.create();
        let a = A("a".to_string());

        let state = AtomicCell::new(ComponentStore::<A>::default());
        state.borrow_mut().insert(e, a.clone());

        let comp = state.borrow();

        assert_eq!(entities.get_single_with(&comp), Ok((e, &a)));
    }

    #[test]
    fn entities__get_single__with_one_required__none() {
        let mut entities = Entities::default();
        (0..3).map(|_| entities.create()).count();

        let state = AtomicCell::new(ComponentStore::<A>::default());
        let comp = state.borrow();

        assert_eq!(
            entities.get_single_with(&comp),
            Err(QuerySingleError::NoEntities)
        );
    }

    #[test]
    fn entities__get_single__with_one_required__too_many() {
        let mut entities = Entities::default();
        let state = AtomicCell::new(ComponentStore::<A>::default());

        for i in 0..3 {
            let e = entities.create();
            let a = A(i.to_string());
            state.borrow_mut().insert(e, a.clone());
        }

        let comp = state.borrow();

        assert_eq!(
            entities.get_single_with(&comp),
            Err(QuerySingleError::MultipleEntities)
        );
    }

    #[test]
    fn entities__get_single__with_multiple_required() {
        let mut entities = Entities::default();

        let state_a = AtomicCell::new(ComponentStore::<A>::default());
        let state_b = AtomicCell::new(ComponentStore::<B>::default());

        let _e1 = entities.create();

        let e2 = entities.create();
        state_a.borrow_mut().insert(e2, A("a2".to_string()));

        let e3 = entities.create();
        state_b.borrow_mut().insert(e3, B("b3".to_string()));

        let e4 = entities.create();
        let a4 = A("a4".to_string());
        let b4 = B("b4".to_string());
        state_a.borrow_mut().insert(e4, a4.clone());
        state_b.borrow_mut().insert(e4, b4.clone());

        let comp_a = state_a.borrow();
        let comp_b = state_b.borrow();
        assert_eq!(
            entities.get_single_with((&comp_a, &comp_b)),
            Ok((e4, (&a4, &b4)))
        );
    }

    #[test]
    fn entities__get_single__with_one_optional() {
        let mut entities = Entities::default();
        let state = AtomicCell::new(ComponentStore::<A>::default());

        {
            let e = entities.create();

            let mut comp = state.borrow_mut();

            assert_eq!(entities.get_single_with(&Optional(&comp)), Ok((e, None)));

            assert_eq!(
                entities.get_single_with(&mut OptionalMut(&mut comp)),
                Ok((e, None))
            );

            entities.kill(e);
        }

        {
            let e = entities.create();
            let mut a = A("a".to_string());
            state.borrow_mut().insert(e, a.clone());

            let mut comp = state.borrow_mut();

            assert_eq!(
                entities.get_single_with(&Optional(&comp)),
                Ok((e, Some(&a)))
            );

            assert_eq!(
                entities.get_single_with(&mut OptionalMut(&mut comp)),
                Ok((e, Some(&mut a)))
            );

            entities.kill(e);
        }
    }

    #[test]
    fn entities__get_single__with_required_and_optional() {
        let mut entities = Entities::default();
        let state_a = AtomicCell::new(ComponentStore::<A>::default());
        let state_b = AtomicCell::new(ComponentStore::<B>::default());

        {
            let e = entities.create();
            let a = A("a".to_string());
            state_a.borrow_mut().insert(e, a.clone());

            let comp_a = state_a.borrow();
            let mut comp_b = state_b.borrow_mut();

            assert_eq!(
                entities.get_single_with((&comp_a, &Optional(&comp_b))),
                Ok((e, (&a, None)))
            );

            assert_eq!(
                entities.get_single_with((&comp_a, &mut OptionalMut(&mut comp_b))),
                Ok((e, (&a, None)))
            );

            entities.kill(e);
        }

        {
            let e = entities.create();
            let a = A("a".to_string());
            let mut b = B("b".to_string());
            state_a.borrow_mut().insert(e, a.clone());
            state_b.borrow_mut().insert(e, b.clone());

            let comp_a = state_a.borrow();
            let mut comp_b = state_b.borrow_mut();

            assert_eq!(
                entities.get_single_with((&comp_a, &Optional(&comp_b))),
                Ok((e, (&a, Some(&b))))
            );

            assert_eq!(
                entities.get_single_with((&comp_a, &mut OptionalMut(&mut comp_b))),
                Ok((e, (&a, Some(&mut b))))
            );

            entities.kill(e);
        }
    }
}

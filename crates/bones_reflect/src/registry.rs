use downcast_rs::{impl_downcast, Downcast};
use hashbrown::{HashMap, HashSet};
use std::any::{Any, TypeId};
use std::fmt::Debug;

pub trait HasTypeRegistration {
    /// Returns the [`std::alloc::Layout`] of the type.
    fn layout() -> std::alloc::Layout;

    /// Returns a pointer to the drop function for the type.
    fn drop_fn() -> Option<unsafe extern "C" fn(*mut u8)>;

    /// Returns a pointer to the clone function for the type.
    fn clone_fn() -> unsafe extern "C" fn(*const u8, *mut u8);

    /// Returns the [type name][std::any::type_name] of the underlying type.
    fn type_name(&self) -> &str;
}

// ===================================== TypeRegistry ===================================== //
pub struct TypeRegistry {
    ambiguous_names: HashSet<String>,
    full_name_to_id: HashMap<String, TypeId>,
    short_name_to_id: HashMap<String, TypeId>,
    registrations: HashMap<TypeId, TypeRegistration>,
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    /// Create a type registry with *no* registered types.
    pub fn empty() -> Self {
        Self {
            registrations: Default::default(),
            full_name_to_id: Default::default(),
            ambiguous_names: Default::default(),
            short_name_to_id: Default::default(),
        }
    }

    /// Create a type registry with default registrations for primitive types.
    pub fn new() -> Self {
        let mut registry = Self::empty();
        // FIXME:
        // registry.register::<bool>();
        // registry.register::<char>();
        // registry.register::<u8>();
        // registry.register::<u16>();
        // registry.register::<u32>();
        // registry.register::<u64>();
        // registry.register::<u128>();
        // registry.register::<usize>();
        // registry.register::<i8>();
        // registry.register::<i16>();
        // registry.register::<i32>();
        // registry.register::<i64>();
        // registry.register::<i128>();
        // registry.register::<isize>();
        // registry.register::<f32>();
        // registry.register::<f64>();
        // registry.register::<String>();
        registry
    }

    /// Registers the type `T`, adding reflect data as specified in the [`Reflect`] derive:
    /// ```rust,ignore
    /// #[derive(Reflect)]
    /// #[reflect(Component, Serialize, Deserialize)] // will register ReflectComponent, ReflectSerialize, ReflectDeserialize
    /// ```
    pub fn register<T>(&mut self)
    where
        T: HasTypeRegistration,
    {
        // FIXME:
        // self.add_registration(T::get_type_registration());
    }

    /// Registers the type described by `registration`.
    pub fn add_registration(&mut self, registration: TypeRegistration) {
        if self.registrations.contains_key(&registration.type_id()) {
            return;
        }

        let short_name = registration.short_name.to_string();
        if self.short_name_to_id.contains_key(&short_name)
            || self.ambiguous_names.contains(&short_name)
        {
            // name is ambiguous. fall back to long names for all ambiguous types
            self.short_name_to_id.remove(&short_name);
            self.ambiguous_names.insert(short_name);
        } else {
            self.short_name_to_id
                .insert(short_name, registration.type_id());
        }
        // FIXME:
        // self.full_name_to_id
        //     .insert(registration.type_name().to_string(), registration.type_id());
        self.registrations
            .insert(registration.type_id(), registration);
    }
}
// ===================================== TypeRegistration ===================================== //

pub struct TypeRegistration {
    short_name: String,
    type_info: &'static TypeInfo,
    data: HashMap<TypeId, Box<dyn TypeData>>,
}

impl Debug for TypeRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeRegistration")
            .field("short_name", &self.short_name)
            .field("type_info", &self.type_info)
            .finish()
    }
}

impl Clone for TypeRegistration {
    fn clone(&self) -> Self {
        let mut data = HashMap::default();
        for (id, type_data) in &self.data {
            data.insert(*id, (*type_data).clone_type_data());
        }

        TypeRegistration {
            data,
            type_info: self.type_info,
            short_name: self.short_name.clone(),
        }
    }
}

impl TypeRegistration {
    /// Returns the [`TypeId`] of the type.
    ///
    /// [`TypeId`]: std::any::TypeId
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_info.type_id()
    }

    /// Returns a reference to the value of type `T` in this registration's type
    /// data.
    ///
    /// Returns `None` if no such value exists.
    pub fn data<T: TypeData>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref())
    }

    /// Returns a mutable reference to the value of type `T` in this
    /// registration's type data.
    ///
    /// Returns `None` if no such value exists.
    pub fn data_mut<T: TypeData>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&TypeId::of::<T>())
            .and_then(|value| value.downcast_mut())
    }

    /// Returns a reference to the registration's [`TypeInfo`]
    pub fn type_info(&self) -> &'static TypeInfo {
        self.type_info
    }

    /// Inserts an instance of `T` into this registration's type data.
    ///
    /// If another instance of `T` was previously inserted, it is replaced.
    pub fn insert<T: TypeData>(&mut self, data: T) {
        self.data.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Returns the [short name] of the type.
    ///
    /// [short name]: bevy_utils::get_short_name
    pub fn short_name(&self) -> &str {
        &self.short_name
    }

    // Returns the [name] of the type.
    //
    // [name]: std::any::type_name
    // pub fn type_name(&self) -> &'static str {
    //     self.type_info.type_name()
    // }
}

// ===================================== Type Info ===================================== //

#[derive(Debug, Clone)]
pub enum TypeInfo {}

impl TypeInfo {}

// ===================================== Type Data ===================================== //

pub trait TypeData: Downcast + Send + Sync {
    fn clone_type_data(&self) -> Box<dyn TypeData>;
}
impl_downcast!(TypeData);

impl<T: 'static + Send + Sync> TypeData for T
where
    T: Clone,
{
    fn clone_type_data(&self) -> Box<dyn TypeData> {
        Box::new(self.clone())
    }
}

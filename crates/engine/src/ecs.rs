use crate::utils::*;
use std::marker;
use std::any;
use std::cmp;
use std::collections;
use std::default::default;
use std::hash;
use std::mem;
use std::ops;
use std::ptr;

pub type Entity = usize;

#[derive(Default)]
pub struct EntityData {
    component_index: ComponentIndex,
    //this is used for dropping and is kind of a hack
    metadata: collections::HashMap<ComponentId, ptr::DynMetadata<dyn Component>>,
    archetype: Option<Archetype>,
}

pub type EntitySlot = Option<EntityData>;

#[derive(Default)]
pub struct Entities {
    slots: Vec<EntitySlot>,
    free: Vec<Entity>,
}

impl Entities {
    fn spawn(&mut self) -> Entity {
        let Some(entity) = self.free.pop() else {
            let entity = self.slots.len();

            self.slots.push(Some(default()));

            return entity;
        };

        self.slots[entity] = Some(default());

        entity
    }

    fn despawn(&mut self, entity: Entity) -> Option<EntityData> {
        if entity > self.slots.len() {
            None?
        }

        let slot = mem::replace(&mut self.slots[entity], None);

        let Some(data) = slot else {
            None?
        };

        self.free.push(entity);

        Some(data)
    }

    fn get(&self, index: usize) -> &EntitySlot {
        &self.slots[index]
    }

    fn get_mut(&mut self, index: usize) -> &mut EntitySlot {
        &mut self.slots[index]
    }
}

impl ops::Index<usize> for Entities {
    type Output = EntityData;

    fn index(&self, index: usize) -> &Self::Output {
        self.slots[index].as_ref().unwrap()
    }
}

impl ops::IndexMut<usize> for Entities {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.slots[index].as_mut().unwrap()
    }
}

pub type ComponentId = any::TypeId;
pub type ComponentIndex = usize;

pub trait Component: 'static {
    fn id() -> ComponentId
    where
        Self: Sized;
    fn size() -> usize
    where
        Self: Sized;
}

impl<T: 'static> Component for T {
    fn id() -> ComponentId {
        ComponentId::of::<T>()
    }

    fn size() -> usize {
        mem::size_of::<T>()
    }
}

#[derive(Default)]
pub struct Components {
    storage: collections::HashMap<Archetype, Storage>,
}

pub type ArchetypeIndex = usize;

#[derive(Clone, Default)]
pub struct Archetype {
    ids: Vec<ComponentId>,
    size: Vec<usize>,
}

impl Archetype {
    fn new() -> Self {
        default()
    }

    fn add<T: Component>(&mut self) {
        self.ids.push(T::id());
        self.ids.sort();
        self.size
            .insert(self.index_of(&T::id()).unwrap(), T::size());
    }

    fn remove_by_index(&mut self, index: usize) -> ComponentId {
        let id = self.ids.remove(index);
        self.size.remove(index);
        id
    }

    fn get_by_index(&self, index: usize) -> ComponentId {
        *self.ids.get(index).unwrap()
    }

    fn index_of(&self, id: &ComponentId) -> Option<usize> {
        self.ids.binary_search(id).ok()
    }

    fn offset_of(&self, index: usize) -> Option<usize> {
        if index > self.size.len() {
            None?
        }
        Some(self.size.iter().cloned().take(index).sum())
    }

    fn size(&self) -> usize {
        self.size.iter().cloned().sum()
    }

    fn len(&self) -> usize {
        self.ids.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl cmp::PartialEq for Archetype {
    fn eq(&self, other: &Self) -> bool {
        self.ids == other.ids
    }
}
impl cmp::Eq for Archetype {}

impl ops::Deref for Archetype {
    type Target = [ComponentId];

    fn deref(&self) -> &Self::Target {
        &self.ids
    }
}

impl hash::Hash for Archetype {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        for id in &self.ids {
            id.hash(state)
        }
    }
}

pub type StorageIndex = usize;

#[derive(Default)]
pub struct Insertion {
    archetype: Archetype,
    entity: Entity,
    data: Vec<u8>,
}

impl Insertion {
    fn new() -> Self {
        default()
    }

    fn add<T: Component>(&mut self, component: T) {
        let component = [component];
        //SAFETY: bytes is used within the lifetime of component
        let bytes = unsafe { to_bytes(&component) };

        self.archetype.add::<T>();
        let index = self.archetype.index_of(&T::id()).unwrap();
        let offset = self.archetype.offset_of(index).unwrap();

        self.data = self
            .data
            .iter()
            .cloned()
            .take(offset)
            .chain(bytes.iter().cloned())
            .chain(self.data.iter().cloned().skip(offset))
            .collect::<Vec<_>>();

        mem::forget(component);
    }

    fn archetype(&self) -> &Archetype {
        &self.archetype
    }

    fn into_data(self) -> Vec<u8> {
        self.data
    }
}

pub struct Swap {
    to: ComponentIndex,
    entity: Entity,
}

pub struct Removal {
    archetype: Archetype,
    data: Vec<u8>,
    swap: Option<Swap>,
}

impl Removal {
    fn remove(&mut self, id: ComponentId) -> Vec<u8> {
        let index = self.archetype.index_of(&id).unwrap();
        let offset = self.archetype.offset_of(index).unwrap();
        let size = self.archetype.size[index];
        let bytes = self.data[offset..offset + size].to_vec();
        self.archetype.remove_by_index(index);
        self.data = self
            .data
            .iter()
            .cloned()
            .take(offset)
            .chain(self.data.iter().cloned().skip(offset + size))
            .collect::<Vec<_>>();
        bytes
    }
}

#[derive(Default)]
pub struct Storage {
    archetype: Archetype,
    data: Vec<u8>,
    entities: Vec<Entity>,
}

impl Storage {
    fn new(archetype: Archetype) -> Self {
        Self {
            archetype,
            data: Vec::with_capacity(50000),
            entities: vec![],
        }
    }

    fn insert(&mut self, insertion: Insertion) -> ComponentIndex {
        if insertion.archetype() != &self.archetype {
            panic!("insertion archetype must match storage archetype");
        }

        let component_index = self.data.len() / self.archetype.size();

        self.entities.push(insertion.entity);
        self.data.extend(insertion.into_data());

        component_index
    }

    fn remove(&mut self, index: ComponentIndex) -> Option<Removal> {
        let size = self.archetype.size();

        if index > self.data.len() / size {
            None?
        }

        let data = self.data[index * size..index * size + size].to_vec();

        let last_index = (self.data.len() / size) - 1;

        let entity = self.entities[index];

        let mut archetype = self.archetype.clone();

        let swap = if index != last_index {
            let from = last_index;
            let to = index;

            let from_data = &self.data[from * size..from * size + size] as *const _ as *const u8;
            let to_data = &mut self.data[to * size..to * size + size] as *mut _ as *mut u8;

            //SAFETY: lifetime of to_data exceeds that of from_data.
            //from_data is not accessed.
            //information stored at to_data has been extracted into data.
            unsafe { ptr::copy(from_data, to_data, size) }

            let entity = self.entities[last_index];

            self.entities.swap_remove(to);

            Some(Swap { to, entity })
        } else {
            self.entities.pop();
            None
        };

        let new_data_len = self.data.len() - size;
        self.data.truncate(new_data_len);

        Some(Removal {
            archetype,
            data,
            swap,
        })
    }

    fn location(&self) -> usize {
        self.data.as_ptr().cast::<u8>() as _
    }
}

#[derive(Default)]
pub struct World {
    entities: Entities,
    components: Components,
}

impl World {
    pub fn new() -> Self {
        default()
    }

    pub fn spawn(&mut self) -> Entity {
        self.entities.spawn()
    }

    pub fn despawn(&mut self, entity: Entity) {
        let Some(mut entity_data) = self.entities.despawn(entity) else {
            return;
        };

        let Some(archetype) = &entity_data.archetype else {
            return;
        };

        let component_index = entity_data.component_index;

        let storage = self.components.storage.get_mut(archetype).unwrap();

        for (id, metadata) in entity_data.metadata.into_iter() {
            let size = archetype.size();
            let index = archetype.index_of(&id).unwrap();
            let offset = archetype.offset_of(index).unwrap();
            let data_location = storage.location() + component_index * size + offset;
            let data_address = data_location as *mut ();
            let ptr = ptr::from_raw_parts_mut::<dyn Component>(data_address, metadata);
            //SAFETY: information at ptr is valid as storage.remove has not been called.
            //data from removal is dropped as a vector of bytes.
            unsafe { ptr.drop_in_place() };
        }

        let mut removal = storage.remove(component_index).unwrap();
    }

    pub fn add<T: Component>(&mut self, entity: Entity, component: T) {
        let Some(entity_data) = self.entities.get_mut(entity) else {
            return;
        };

        let Some(old_archetype) = &entity_data.archetype else {
            let mut insertion = Insertion::new();
            insertion.add(component);
            
            let mut new_storage = self
                .components
                .storage
                .entry(insertion.archetype().clone())
                .or_insert(Storage::new(insertion.archetype().clone()));
            
            entity_data.component_index = new_storage.entities.len();
            entity_data.archetype = Some(insertion.archetype().clone());

            new_storage.insert(insertion);
            
            return;
        };

        let old_archetype = old_archetype.clone();

        let component_index = entity_data.component_index;

        let old_storage = self.components.storage.get_mut(&old_archetype).unwrap();

        let old_storage_location = old_storage.location();

        let mut removal = old_storage.remove(component_index).unwrap();

        let Removal {
            archetype,
            data,
            swap,
        } = removal;

        if archetype.is_empty() {
            entity_data.component_index = 0;
            entity_data.archetype = None;
        }

        let mut insertion = Insertion {
            archetype,
            entity,
            data,
        };

        insertion.add(component);

        let new_archetype = insertion.archetype().clone();

        let new_storage = self
            .components
            .storage
            .entry(new_archetype.clone())
            .or_insert(Storage::new(insertion.archetype().clone()));

        let old_component_index = entity_data.component_index;
        
        entity_data.component_index = new_storage.entities.len();
        entity_data.archetype = Some(new_archetype.clone());

        new_storage.insert(insertion);

        let new_storage_location = new_storage.location();

        let size = new_archetype.size();
        let index = new_archetype.index_of(&T::id()).unwrap();
        let offset = new_archetype.offset_of(index).unwrap();
        let data_location = new_storage.location() + entity_data.component_index * size + offset;
        let data_address = data_location as *const T;
        let metadata = ptr::metadata::<dyn Component>(data_address);
        entity_data.metadata.insert(T::id(), metadata);

        drop(entity_data);
        drop(new_storage);
        let old_storage = self.components.storage.get(&old_archetype).unwrap();

        let old_storage_location = old_storage.location();

        if let Some(Swap { entity, to }) = swap {
            self.entities[entity].component_index = to;
            let old_archetype = self.entities[entity].archetype.as_ref().unwrap().clone();
        }

        for &entity in &old_storage.entities {
            if self.entities[entity].component_index > old_component_index {
                self.entities[entity].component_index -= 1;
            }
        }
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let Some(entity_data) = self.entities.get_mut(entity) else {
            None?
        };

        let Some(old_archetype) = &entity_data.archetype else {
            None?
        };

        let old_archetype = old_archetype.clone();

        if !old_archetype.contains(&T::id()) {
            None?
        }

        let component_index = entity_data.component_index;


        let old_storage = self.components.storage.get_mut(&old_archetype).unwrap();

        let mut removal = old_storage.remove(component_index).unwrap();

        let bytes = removal.remove(T::id());

        entity_data.metadata.remove(&T::id());

        let Removal {
            archetype,
            data,
            swap,
        } = removal;

        if archetype.is_empty() {
            entity_data.component_index = 0;
            entity_data.archetype = None;
        }

        let mut insertion = Insertion {
            archetype,
            entity,
            data,
        };

        let new_archetype = insertion.archetype().clone();

        let new_storage = self
            .components
            .storage
            .entry(new_archetype.clone())
            .or_insert(Storage::new(insertion.archetype().clone()));

        let old_component_index = entity_data.component_index;

        entity_data.component_index = new_storage.entities.len();
        entity_data.archetype = Some(new_archetype.clone());

        new_storage.insert(insertion);

        let new_storage_location = new_storage.location();

        drop(entity_data);
        drop(new_storage);
        let old_storage = self.components.storage.get(&old_archetype).unwrap();

        let old_storage_location = old_storage.location();

        if let Some(Swap { entity, to }) = swap {
            self.entities[entity].component_index = to;
        }

        for &entity in &old_storage.entities {
            if self.entities[entity].component_index > old_component_index {
                self.entities[entity].component_index -= 1;
            }
        }

        //SAFETY: lifetime of bytes is extended to that of T
        //since there is no more information where bytes came from (in storage),
        //it is safe to cast it back to its original type
        Some(unsafe { from_bytes(&bytes) })
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let mut entities = vec![];
        for (entity, slot) in self.entities.slots.iter().enumerate() {
            if let Some(_) = slot {
                entities.push(entity);
            }
        }
        for entity in entities {
            self.despawn(entity);
        }
    }
}

pub struct Schedule {

}

impl Schedule {
    pub fn new() -> Self {
        todo!()
    }

    pub fn add_stage(&mut self, stage: Stage) -> &mut Self {
        todo!()
    }
}

pub struct Stage {
    systems: Vec<BoxedSystem>,
}

impl Stage {
    pub fn serial() -> Self {
        todo!()
    }

    pub fn parallel() -> Self {
        todo!()
    }

    pub fn add_system<Params>(mut self, system: impl IntoSystem<(), (), Params>) -> Self 
    {
        let system = box IntoSystem::into_system(system);
        
        self.systems.push(system);
        
        self
    }
}

pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;

pub trait System: 'static + Send + Sync {
    type In;
    type Out;

    fn call(&mut self, input: Self::In) -> Self::Out;
    fn fetch_input(&mut self, world: &mut World, resources: &mut Resources);
    fn take_output(&mut self) -> Self::Out;
}

pub struct SystemDescriptor {
    
}

pub trait IntoSystem<In, Out, Params>: 'static {
    type System: System<In = In, Out = Out>;

    fn into_system(this: Self) -> Self::System;
}

pub struct AlreadyWasSystem;

impl<A, B, C> IntoSystem<B, C, AlreadyWasSystem> for A
    where A: System<In = B, Out = C>
{
    type System = A;

    fn into_system(this: Self) -> Self::System {
        this
    }
}

pub struct FunctionSystem<Function, In, Out> {
    function: Function,
    input: Option<In>,
    output: Option<Out>,
    marker: marker::PhantomData<(In, Out)>,
}

pub struct IsFunctionSystem<In, Out> {
    marker: marker::PhantomData<(In, Out)>,
}

impl<A, B, C> IntoSystem<(), (), IsFunctionSystem<B, C>> for A 
    where A: 'static + Fn(B) -> C + Send + Sync,
          B: 'static + SystemParameter,
          C: 'static + SystemParameter
{
    type System = FunctionSystem<A, (B,), C>;

    fn into_system(function: Self) -> Self::System {
        FunctionSystem {
            function, 
            input: None,
            output: None,
            marker: marker::PhantomData,
        }
    }
}

impl<A, B, C> System for FunctionSystem<A, (B,), C>
    where A: 'static + Fn(B) -> C + Send + Sync,
          B: 'static + Send + Sync,
          C: 'static + Send + Sync
{
    type In = ();
    type Out = ();

    fn call(&mut self, _: Self::In) -> Self::Out {
        self.output = self.input.take().map(|input| (self.function)(input.0));  
    }
}

pub trait SystemParameter: Send + Sync {
}

impl<Q: QueryParameter> SystemParameter for Query<Q> {

}

impl SystemParameter for () {
    
}

impl<A> SystemParameter for (A,) 
    where A: SystemParameter
{

}

impl<A, B> SystemParameter for (A, B) 
    where A: SystemParameter,
          B: SystemParameter,
{

}

pub struct Query<Q: QueryParameter> {
    marker: marker::PhantomData<Q>, 
}

pub trait QueryParameter: Send + Sync {
    
}

impl QueryParameter for Entity {

}

impl<'a, T> QueryParameter for &'a T 
    where T: 'static + Send + Sync
{
    
}

impl<'a, T> QueryParameter for &'a mut T
    where T: 'static + Send + Sync
{

}

impl<A> QueryParameter for (A,) 
    where A: QueryParameter
{

}

impl<A, B> QueryParameter for (A, B) 
    where A: QueryParameter,
          B: QueryParameter,
{

}

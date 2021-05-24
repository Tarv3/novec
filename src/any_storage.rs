use std::any::Any;

pub trait Component {
    type Storage;
}

pub trait ComponentStorage {
    fn get<T: Any + Component>(&self) -> Option<&T::Storage>;
    fn get_mut<'a, T: 'a + Any + Component>(&'a mut self) -> Option<&'a mut T::Storage>;

    fn get_unchecked<T: Any + Component>(&self) -> &T::Storage {
        self.get::<T>().unwrap()
    }

    fn get_mut_unchecked<'a, T: 'a + Any + Component>(&'a mut self) -> &'a mut T::Storage {
        self.get_mut::<T>().unwrap()
    }
}

#[macro_export]
macro_rules! create_storage {
    ($name:ident { $($component:ident : $component_type:ty),+ }) => {
        pub struct $name {
            $(
                pub $component: <$component_type as Component>::Storage,
            )+
        }

        impl ComponentStorage for $name {
            fn get<T: Any + Component>(&self) -> Option<&T::Storage> {
                use std::any::TypeId;
                unsafe {
                    match TypeId::of::<T>() {
                        $(
                            x if x == TypeId::of::<$component_type>() => Some(std::mem::transmute(&self.$component)),
                        )+
                        _ => None
                    }
                }
            }

            fn get_mut<'a, T: 'a + Any + Component>(&'a mut self) -> Option<&'a mut T::Storage> {
                use std::any::TypeId;
                unsafe {
                    match TypeId::of::<T>() {
                        $(
                            x if x == TypeId::of::<$component_type>() => Some(std::mem::transmute(&mut self.$component)),
                        )+
                        _ => None
                    }
                }
            }
        }
    };
    ($name:ident < $($generic:ident),+ > { $($component:ident : $component_type:ty),+ }) => {
        pub struct $name <$(generic),+> {
            $(
                pub $component: <$component_type as Component>::Storage,
            )+
        }

        impl<$(generic),+> ComponentStorage for $name<$(generic),+> {
            fn get<T: Any + Component>(&self) -> Option<&T::Storage> {
                use std::any::TypeId;
                unsafe {
                    match TypeId::of::<T>() {
                        $(
                            x if x == TypeId::of::<$component_type>() => Some(std::mem::transmute(&self.$component)),
                        )+
                        _ => None
                    }
                }
            }

            fn get_mut<'a, T: 'a + Any + Component>(&'a mut self) -> Option<&'a mut T::Storage> {
                use std::any::TypeId;
                unsafe {
                    match TypeId::of::<T>() {
                        $(
                            x if x == TypeId::of::<$component_type>() => Some(std::mem::transmute(&mut self.$component)),
                        )+
                        _ => None
                    }
                }
            }
        }
    };
}

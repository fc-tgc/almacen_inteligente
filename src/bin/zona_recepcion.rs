use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;
use std::thread;
use std::env;

const CAPACIDAD_CINTA: usize = 10;

struct Paquete {
    id: usize,
}

struct EstadoCinta { 
    buffer: VecDeque<Paquete>,
    paquetes_restantes: usize,
 }
struct ZonaAlmacenamiento { 
    camiones: usize,
    robots: usize,
    hay_paquetes: Condvar,
    cinta_llena: Condvar,
    estado: Mutex<EstadoCinta>,
 }

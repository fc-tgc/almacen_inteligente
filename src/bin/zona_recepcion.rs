use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;
use std::thread;
use std::env;

const CAPACIDAD_CINTA: usize = 10;

struct Paquete {
    id: usize,
    // faltan un par de campos en caso que tenga que ser consistente con la parte 1 del TP.
}

struct EstadoCinta { 
    buffer: VecDeque<Paquete>,
    paquetes_restantes: usize,
    paquetes_procesados: usize,
 }
struct ZonaAlmacenamiento { 
    camiones: usize,
    robots: usize,
    hay_paquetes: Condvar,
    cinta_llena: Condvar,
    estado: Mutex<EstadoCinta>,
 }

 fn rol_camion(id: usize, zona: Arc<ZonaAlmacenamiento>) {
    loop {
        let mut guard = zona.estado.lock().unwrap();
        if guard.paquetes_restantes == 0 {
            break;
        }
        guard = zona.cinta_llena.wait_while(guard, |e| e.buffer.len() == CAPACIDAD_CINTA).unwrap();
        guard.paquetes_restantes -= 1;
        guard.paquetes_procesados += 1;
        let paquete = Paquete{id: guard.paquetes_procesados};
        let id_paquete = paquete.id;
        guard.buffer.push_back(paquete);
        zona.hay_paquetes.notify_one();
        drop(guard);
        println!("Camión {}: dejó paquete {}", id, id_paquete);
    }
 }
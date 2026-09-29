use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

const CANTIDAD_BAHIAS: usize = 4;
const DURACION_REPARACION: Duration = Duration::from_millis(20);
const DURACION_TRABAJO: Duration = Duration::from_millis(10);

/// Los tres desenlaces de un pedido de mantenimiento. El usize es la
/// cantidad de reparaciones que van a terminar antes que la del robot que pidió
/// el turno (ver `pedir_mantenimiento`).
enum Ingreso {
    /// El robot fue atendido de inmediato.
    DespertarMecanico(usize),
    /// El robot fue puesto a esperar en una bahía.
    OcuparBahia(usize),
    /// El robot fue rechazado porque no había bahía libre. Vuelve a trabajar sin esperar.
    Rechazado,
}

/// Estado compartido por el Mecánico y robots que se cubre con un Mutex
struct EstadoMantenimiento {
    /// Robot que el Mecánico está reparando en este instante, si hay alguno.
    reparacion_actual: Option<usize>,
    /// Robots que ocupan unabahía de espera. El primero de la fila es el próximo a ser reparado.
    bahias: VecDeque<usize>,
    /// Cantidad de reparaciones ya terminadas y turnero para los robots
    completadas: usize,
    /// Cantidad de robots que todavía tienen visitas pendientes. Cuando llega a 0, el Mecánico puede terminar la simulación.
    robots_activos: usize,
}

/// Recursos compartidos entre el hilo del Mecánico y los hilos de los robots.
/// Las Condvar viven fuera del Mutex porque un hilo necesita poder tomar el lock
/// primero y recién después esperar sobre la condición pasándole el guard.
struct EstacionMantenimiento {
    robots: usize,
    /// Espera el Mecánico. Se avisa cuando llega un robot a la puerta o cuando
    /// un robot termina su última visita (para que el Mecánico pueda cerrar).
    hay_robots: Condvar,
    /// Esperan los robots. Se avisa cuando termina una reparación.
    reparacion_terminada: Condvar,
    estado: Mutex<EstadoMantenimiento>,
}

/// Rol del Mecánico: duerme mientras no haya robots esperando y repara al primero
/// que se acerque. Es el único hilo que repara, así que nunca hay dos
/// reparaciones en paralelo.
fn rol_mecanico(estacion: Arc<EstacionMantenimiento>) {
    loop {
        let mut guard = estacion.estado.lock().unwrap();

        // 1. Duerme mientras no haya trabajo. Este `wait` *es* el "dormir".
        guard = estacion
            .hay_robots
            .wait_while(guard, |e| {
                e.robots_activos > 0 && e.reparacion_actual.is_none() && e.bahias.is_empty()
            })
            .unwrap();

        // 2. Sin robots con visitas pendientes terminó la simulación.
        if guard.robots_activos == 0 {
            break;
        }

        // 3. Atiende a un robot: primero el que lo despertó, si lo hay; si no, el
        //    primero de la fila. Marcarlo ocupado junto con sacarlo de la fila es
        //    lo que impide que otro robot lo crea durmiendo y se lleve su turno.
        let id = if let Some(id) = guard.reparacion_actual {
            id
        } else {
            let id = guard
                .bahias
                .pop_front()
                .expect("había robots activos, entonces alguno espera");
            guard.reparacion_actual = Some(id);
            id
        };
        drop(guard);

        println!("Mecánico: repara al Robot {}", id);
        thread::sleep(DURACION_REPARACION);

        // 4. Termina la reparación. Avisa a todos y no a uno: cada robot reevalúa
        //    el reloj al despertar, y con `notify_one` se podría despertar al
        //    robot equivocado y dejar al reparado esperando para siempre.
        let mut guard = estacion.estado.lock().unwrap();
        guard.reparacion_actual = None;
        guard.completadas += 1;
        estacion.reparacion_terminada.notify_all();
    }
}

/// Rol de un robot: pide mantenimiento `visitas` veces, esperando entre pedido y
/// pedido el tiempo que simula estar trabajando.
fn rol_robot(id: usize, visitas: usize, estacion: Arc<EstacionMantenimiento>) {
    for _ in 0..visitas {
        thread::sleep(DURACION_TRABAJO);
        let por_delante = match pedir_mantenimiento(id, &estacion) {
            Ingreso::Rechazado => continue,
            Ingreso::DespertarMecanico(n) | Ingreso::OcuparBahia(n) => n,
        };
        esperar_reparacion(por_delante, &estacion);
        println!(
            "Robot {}: reparación finalizada, vuelve al sector de trabajo",
            id
        );
    }

    // Ya no tiene visitas pendientes: el Mecánico puede terminar si es el último.
    let mut guard = estacion.estado.lock().unwrap();
    guard.robots_activos -= 1;
    drop(guard);
    estacion.hay_robots.notify_one();
}

/// Un robot llega a la estación y la clasifica según el estado del Mecánico y de
/// las bahías. En los dos casos primeros lo deja esperando el final de su
/// reparación; en el tercero lo rechaza y vuelve a trabajar.
fn pedir_mantenimiento(id: usize, estacion: &EstacionMantenimiento) -> Ingreso {
    // Leer el estado, decidir y modificarlo tienen que ser atómicos: si no, dos
    // robots podrían "despertar" al Mecánico dormido al mismo tiempo.
    let ingreso = {
        let mut guard = estacion.estado.lock().unwrap();

        // La fila es, en orden: el robot en el taller, los que ya ocupan bahía y
        // el que está llegando. Se calcula **antes** de modificar el estado, con
        // el largo de la fila que había al momento de llegar.
        let por_delante = guard.completadas
            + guard.bahias.len()
            + usize::from(guard.reparacion_actual.is_some());

        if guard.reparacion_actual.is_none() && guard.bahias.is_empty() {
            // Caso 1: el Mecánico está durmiendo. El robot lo reserva para sí y lo despierta
            guard.reparacion_actual = Some(id);
            drop(guard);
            estacion.hay_robots.notify_one();
            Ingreso::DespertarMecanico(por_delante)
        } else if guard.bahias.len() < CANTIDAD_BAHIAS {
            // Caso 2: el Mecánico está ocupado pero hay bahía libre.
            guard.bahias.push_back(id);
            drop(guard);
            // Aviso defensivo: en la práctica el Mecánico está reparando y ya va
            // a volver a mirar la fila al terminar.
            estacion.hay_robots.notify_one();
            Ingreso::OcuparBahia(por_delante)
        } else {
            // Caso 3: el Mecánico está ocupado y las bahías están llenas.
            Ingreso::Rechazado
        }
    };

    // Se imprime con el lock liberado: imprimir es lento y bloquearía a todos.
    match ingreso {
        Ingreso::DespertarMecanico(_) => {
            println!("Robot {}: llega a mantenimiento y despierta al mecánico", id)
        }
        Ingreso::OcuparBahia(_) => println!("Robot {}: ocupa una bahía de espera", id),
        Ingreso::Rechazado => println!("Robot {} rechazado, vuelve a trabajar", id),
    }

    ingreso
}

/// Bloquea al robot hasta que el Mecánico termine de repararlo: su reparación es
/// la número `por_delante + 1`, o sea, hay que esperar a que terminen todas las
/// anteriores y la propia.
fn esperar_reparacion(por_delante: usize, estacion: &EstacionMantenimiento) {
    let mut guard = estacion.estado.lock().unwrap();
    while guard.completadas <= por_delante {
        guard = estacion.reparacion_terminada.wait(guard).unwrap();
    }
}

/// Imprime cómo se usa el programa y termina con error. Se usa cuando falta un
/// argumento o cuando no es un número.
fn error_de_argumentos(mensaje: &str) -> ! {
    eprintln!("Error: {}", mensaje);
    eprintln!();
    eprintln!("Uso: cargo run --bin mantenimiento -- <robots> <visitas_por_robot>");
    eprintln!("  <robots>            cantidad de robots que participan de la simulación");
    eprintln!("  <visitas_por_robot> cuántos pedidos de mantenimiento hace cada robot");
    eprintln!();
    eprintln!("Ejemplo: cargo run --bin mantenimiento -- 6 3");
    std::process::exit(1)
}

/// Lee el argumento de la posición `indice` como entero positivo. Si falta o no
/// se puede convertir, termina con un mensaje de error en vez de hacer panic con
/// un error de índice.
fn leer_argumento(args: &[String], indice: usize, nombre: &str) -> usize {
    let Some(texto) = args.get(indice) else {
        error_de_argumentos(&format!("falta el argumento <{}>", nombre));
    };
    match texto.parse::<usize>() {
        Ok(numero) => numero,
        Err(_) => error_de_argumentos(&format!(
            "el argumento <{}> debe ser un entero positivo, se recibió \"{}\"",
            nombre, texto
        )),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cantidad_robots = leer_argumento(&args, 1, "robots");
    let visitas_por_robot = leer_argumento(&args, 2, "visitas_por_robot");

    let estado_inicial = EstadoMantenimiento {
        reparacion_actual: None,
        bahias: VecDeque::new(),
        completadas: 0,
        robots_activos: cantidad_robots,
    };

    let estacion = Arc::new(EstacionMantenimiento {
        robots: cantidad_robots,
        hay_robots: Condvar::new(),
        reparacion_terminada: Condvar::new(),
        estado: Mutex::new(estado_inicial),
    });

    let mut handles = Vec::new();

    let estacion_clon = Arc::clone(&estacion);
    handles.push(thread::spawn(move || {
        rol_mecanico(estacion_clon);
    }));

    for id in 0..estacion.robots {
        let estacion_clon = Arc::clone(&estacion);
        handles.push(thread::spawn(move || {
            rol_robot(id, visitas_por_robot, estacion_clon);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

# Almacén Inteligente

Implementación en Rust del TP N°1 de Programación Concurrente (FIUBA): dos subsistemas concurrentes independientes, cada uno como un binario separado dentro del mismo proyecto.

- **`zona_recepcion`** — Módulo A: camiones (productores) dejan paquetes en una cinta transportadora de capacidad acotada y robots (consumidores) los retiran para procesarlos.
- **`mantenimiento`** — Módulo B.

## Requisitos

- Rust y Cargo instalados.

## Compilación

Desde la raíz del proyecto:

```bash
cargo build
```

Al tratarse de un proyecto con múltiples binarios (`src/bin/zona_recepcion.rs` y `src/bin/mantenimiento.rs`), `cargo build` compila ambos en una sola ejecución. Debería compilar **sin warnings**.

## Ejecución

Cada binario se ejecuta por separado indicando su nombre con `--bin`. `zona_recepcion` recibe un único argumento: la cantidad total de paquetes a producir.

```bash
cargo run --bin zona_recepcion -- 20
```

```bash
cargo run --bin mantenimiento
```

## Plan de pruebas

El objetivo de estas pruebas es verificar, antes de dar cada programa por terminado, que **no tiene deadlocks** y que **finaliza correctamente**. Se asume sistema operativo Linux para los ejemplos a continuación.

### Módulo A — Zona de Recepción

#### 1. Variación de volumen

Correr con distintas cantidades de paquetes:

```bash
cargo run --bin zona_recepcion -- 1
cargo run --bin zona_recepcion -- 5
cargo run --bin zona_recepcion -- 500
cargo run --bin zona_recepcion -- 5000
```

**Resultado esperado:** en todos los casos el proceso termina y vuelve a liberar el input de la terminal.

#### 2. Verificación de conteo

Confirmar que no se pierde ni se duplica ningún paquete: la cantidad de entregas debe ser igual a la cantidad de retiros, y ambas iguales a `N`. Reemplazar en N por un número entero positivo en la terminal.

```bash
cargo run --bin zona_recepcion -- N > /tmp/salida.txt
grep -c "dejó paquete" /tmp/salida.txt
grep -c "tomó paquete" /tmp/salida.txt
```

**Resultado esperado:** ambos comandos `grep -c` devuelven `N`.

#### 3. Repetición para detectar no-determinismo

Como es un problema de scheduling concurrente, un bug de sincronización puede no manifestarse en una sola corrida. Se repite la misma configuración varias veces, con un `timeout` que detecta si alguna corrida se cuelga:

```bash
N=500
CORRIDAS=30

for i in $(seq 1 $CORRIDAS); do
  if timeout 10 cargo run --bin zona_recepcion -- $N > /tmp/salida_$i.txt 2>&1; then
    entregados=$(grep -c "dejó paquete" /tmp/salida_$i.txt)
    tomados=$(grep -c "tomó paquete" /tmp/salida_$i.txt)
    if [ "$entregados" -eq "$N" ] && [ "$tomados" -eq "$N" ]; then
      echo "Corrida $i: OK ($entregados/$tomados)"
    else
      echo "Corrida $i: FALLÓ conteo (entregados=$entregados tomados=$tomados)"
    fi
  else
    echo "Corrida $i: TIMEOUT (posible deadlock)"
  fi
done
```

**Resultado esperado:** las 30 corridas terminan dentro del timeout (10s) e imprimen `Corrida #: OK (500/500)`, donde # corresponde al número de corrida. Ninguna debería reportar `TIMEOUT` ni un desajuste de conteo.

#### 4. Caso límite: `N = 0`

Verificar que, sin paquetes que producir, el programa no se cuelga esperando algo que nunca va a llegar:

```bash
cargo run --bin zona_recepcion -- 0
```

**Resultado esperado:** el programa termina inmediatamente, sin imprimir ninguna línea de `"dejó paquete"` ni `"tomó paquete"` (los camiones no tienen nada que producir, y los robots detectan de inmediato que no hay ni buffer con paquetes ni producción pendiente).

### Módulo B — Estación de Mantenimiento
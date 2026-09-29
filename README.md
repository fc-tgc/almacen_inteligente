# Almacén Inteligente

Implementación en Rust del TP N°1 de Programación Concurrente (FIUBA): dos subsistemas concurrentes independientes, cada uno como un binario separado dentro del mismo proyecto.

- **`zona_recepcion`** — Módulo A: camiones (productores) dejan paquetes en una cinta transportadora de capacidad acotada y robots (consumidores) los retiran para procesarlos.
- **`mantenimiento`** — Módulo B: 1 mecánico y 4 bahías de espera. Los robots van a la estación de mantenimiento y, según si el mecánico está durmiendo, ocupado o saturado, lo despiertan, esperan una bahía o son rechazados (el problema del "barbero dormilón").

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

`mantenimiento` recibe dos argumentos: la cantidad de robots que participan de la simulación y la cantidad de visitas (pedidos de mantenimiento) que hace cada robot. Las visitas son el límite de la simulación: cuando el último robot termina su última visita, el mecánico deja de dormir y el programa termina.

```bash
cargo run --bin mantenimiento -- 6 3
```

Si falta un argumento o no es un número entero, el programa imprime el modo de uso en `stderr` y termina con código de salida `1`:

```text
$ cargo run --bin mantenimiento
Error: falta el argumento <robots>

Uso: cargo run --bin mantenimiento -- <robots> <visitas_por_robot>
  <robots>            cantidad de robots que participan de la simulación
  <visitas_por_robot> cuántos pedidos de mantenimiento hace cada robot

Ejemplo: cargo run --bin mantenimiento -- 6 3
```

Los mensajes que imprime el módulo B, y que usan las pruebas de abajo para verificar el comportamiento:

| Mensaje | Significado |
| --- | --- |
| `Robot X: llega a mantenimiento y despierta al mecánico` | Caso 1: el mecánico dormía, el robot lo despertó. |
| `Robot X: ocupa una bahía de espera` | Caso 2: el mecánico estaba ocupado y había bahía libre. |
| `Robot X rechazado, vuelve a trabajar` | Caso 3: el mecánico estaba ocupado y las 4 bahías estaban llenas. |
| `Mecánico: repara al Robot X` | El mecánico comienza una reparación. |
| `Robot X: reparación finalizada, vuelve al sector de trabajo` | Terminó la reparación de `X`; el robot vuelve a trabajar. |

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

#### 1. Variación de volumen

Correr con distintas cantidades de robots y de visitas por robot:

```bash
cargo run --bin mantenimiento -- 1 1
cargo run --bin mantenimiento -- 3 2
cargo run --bin mantenimiento -- 6 3
cargo run --bin mantenimiento -- 20 5
```

**Resultado esperado:** en todos los casos el proceso termina y vuelve a liberar el input de la terminal. Con menos de 6 robots casi no hay rechazos (el mecánico alcanza a reparar antes de que se llenen las 4 bahías); con 20 robots y 5 visitas las bahías se llenan seguido y se ven muchos rechazos, que es justamente lo que se quiere provocar.

#### 2. Verificación de conteo

Cada visita de cada robot tiene que terminar en exactamente uno de los tres desenlaces, y cada robot que entró a la estación tiene que ser reparado una sola vez. Se comprueban tres igualdades:

- `directas + bahías + rechazos = robots × visitas`: no se pierde ni se duplica ninguna visita.
- `reparaciones = directas + bahías`: toda visita admitida terminó en una reparación.
- `finalizadas = reparaciones`: ningún robot se despertó antes de que lo repararan.

```bash
R=6
V=3

cargo run --bin mantenimiento -- $R $V > /tmp/salida.txt

directas=$(grep -c "despierta al mecánico" /tmp/salida.txt)
bahias=$(grep -c "ocupa una bahía de espera" /tmp/salida.txt)
rechazados=$(grep -c "rechazado, vuelve a trabajar" /tmp/salida.txt)
reparaciones=$(grep -c "Mecánico: repara al Robot" /tmp/salida.txt)
finalizadas=$(grep -c "reparación finalizada" /tmp/salida.txt)

if [ $((directas + bahias + rechazados)) -eq $((R * V)) ] &&
   [ "$reparaciones" -eq $((directas + bahias)) ] &&
   [ "$finalizadas" -eq "$reparaciones" ]; then
  echo "OK: $((R * V)) visitas = $directas directas + $bahias en bahía + $rechazados rechazos, $reparaciones reparaciones"
else
  echo "FALLÓ: directas=$directas bahías=$bahias rechazos=$rechazados reparaciones=$reparaciones finalizadas=$finalizadas"
fi
```

**Resultado esperado:** `OK`, con las tres igualdades cumplidas. La cantidad de rechazos **no** es un valor fijo: depende de cómo se repartan las llegadas, y lo que tiene que cumplirse siempre son las igualdades.

#### 3. Repetición para detectar no-determinismo

Como es un problema de scheduling concurrente, un bug de sincronización puede no manifestarse en una sola corrida. Se repite la misma configuración varias veces, con un `timeout` que detecta si alguna corrida se cuelga:

```bash
R=6
V=3
CORRIDAS=30

for i in $(seq 1 $CORRIDAS); do
  if timeout 10 cargo run --bin mantenimiento -- $R $V > /tmp/salida_$i.txt 2>&1; then
    directas=$(grep -c "despierta al mecánico" /tmp/salida_$i.txt)
    bahias=$(grep -c "ocupa una bahía de espera" /tmp/salida_$i.txt)
    rechazados=$(grep -c "rechazado, vuelve a trabajar" /tmp/salida_$i.txt)
    reparaciones=$(grep -c "Mecánico: repara al Robot" /tmp/salida_$i.txt)
    if [ $((directas + bahias + rechazados)) -eq $((R * V)) ] && [ "$reparaciones" -eq $((directas + bahias)) ]; then
      echo "Corrida $i: OK ($reparaciones reparaciones, $rechazados rechazos)"
    else
      echo "Corrida $i: FALLÓ conteo (directas=$directas bahías=$bahias rechazos=$rechazados reparaciones=$reparaciones)"
    fi
  else
    echo "Corrida $i: TIMEOUT (posible deadlock)"
  fi
done
```

**Resultado esperado:** las 30 corridas terminan dentro del timeout (10s) e imprimen `Corrida #: OK (...)`, donde # corresponde al número de corrida. Ninguna debería reportar `TIMEOUT` ni un desajuste de conteo.

#### 4. Un solo robot

Con un único robot la corrida es determinista: nunca hay otro robot esperando, así que siempre entra por el caso 1 y nunca es rechazado.

```bash
cargo run --bin mantenimiento -- 1 5 > /tmp/salida.txt
grep -c "despierta al mecánico" /tmp/salida.txt
grep -c "ocupa una bahía de espera" /tmp/salida.txt
grep -c "rechazado, vuelve a trabajar" /tmp/salida.txt
```

**Resultado esperado:** `5`, `0` y `0`, respectivamente.

#### 5. Casos límite

Verificar que el programa no se cuelga esperando algo que nunca va a llegar cuando no hay nada que hacer:

```bash
cargo run --bin mantenimiento -- 0 3
cargo run --bin mantenimiento -- 3 0
```

**Resultado esperado:** ambos terminan inmediatamente, sin imprimir ninguna línea. En el primero no hay ningún robot en la estación, así que el mecánico no tiene a quién esperar; en el segundo los robots no tienen visitas pendientes, así que no llegan nunca.
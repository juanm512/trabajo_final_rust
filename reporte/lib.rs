#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod reporte {
    use core::ops::Mul;
    use ink::prelude::string::ToString;
    use ink::prelude::vec::Vec;
    use scale_info::prelude::string::String;
    use scale_info::prelude::vec;

    use sistema_elecciones::SistemaEleccionesRef;

    #[ink(storage)]
    pub struct Reporte {
        administrador: AccountId,
        sistema_elecciones: Option<SistemaEleccionesRef>,
    }

    #[ink(impl)]
    impl Reporte {
        /// Constructor that initializes `sistema_elecciones` to `None`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                administrador: Self::env().caller(),
                sistema_elecciones: None,
            }
        }

        /// Function to set `sistema_elecciones`.
        #[ink(message)]
        pub fn set_sistema_elecciones(
            &mut self,
            sistema_elecciones: SistemaEleccionesRef,
        ) -> Result<String, String> {
            if self.env().caller() != self.administrador {
                return Err("No sos el administrador".to_string());
            }
            self.sistema_elecciones = Some(sistema_elecciones);
            return Ok("Sistema elecciones guardado correctamente!".to_string());
        }

        /// Utilizado por todos los usuarios.
        /// Obtiene la lista de votantes para una elección específica.
        /// Parametors:
        ///     id_eleccion: u64: ID de la elección.
        /// Retorno:
        ///     Result<Vec<(AccountId, String, String, String)>, String>: Vector con el ID de cada votante y su información detallada, o un mensaje de error.
        /// Descripción:
        /// La función recupera la lista de votantes de una elección dada por su ID (`id_eleccion`). Llama a una función privada
        /// para obtener los datos y luego añade información detallada sobre cada votante. Retorna un vector de tuplas con
        /// el `AccountId`, nombre, dirección y otros detalles del votante, o un mensaje de error en caso de fallo.
        #[ink(message)]
        pub fn reporte_de_votantes_por_eleccion(
            &mut self,
            id_eleccion: u64,
        ) -> Result<Vec<(AccountId, String, String, String)>, String> {
            let sistema_elecciones = match &mut self.sistema_elecciones {
                None => return Err("Sistema elecciones no seteado".to_string()),
                Some(value) => value,
            };
            let datos_votantes =
                match sistema_elecciones.obtener_votantes_eleccion_por_id(id_eleccion) {
                    Err(msg) => return Err(msg),
                    Ok(datos) => datos,
                };

            Ok(datos_votantes
                .iter()
                .map(|datos_votante| {
                    let datos_usuario = sistema_elecciones
                        .obtener_informacion_usuario(datos_votante.0)
                        .unwrap_or_default();
                    (
                        datos_votante.0.clone(),
                        datos_usuario.0,
                        datos_usuario.1,
                        datos_usuario.2,
                    )
                })
                .collect())
        }

        /// Obtiene la participación en una elección específica.
        /// Parametros:
        ///     id_eleccion: u64: ID de la elección.
        /// Retorno:
        ///     Result<(u32, u32), String>: Una tupla con la cantidad de votantes efectivos y el porcentaje de participación, o un mensaje de error.
        /// Descripción:
        /// La función recupera la participación en una elección indicada por `id_eleccion`. Llama a una función privada
        /// para obtener los datos de los votantes. Calcula el número de votantes que participaron efectivamente y el
        /// porcentaje de participación. Devuelve estos valores en una tupla, o un mensaje de error si falla.
        #[ink(message)]
        pub fn reporte_de_participacion_por_eleccion(
            &mut self,
            id_eleccion: u64,
        ) -> Result<(u32, u32), String> {
            let sistema_elecciones = match &mut self.sistema_elecciones {
                None => return Err("Sistema elecciones no seteado".to_string()),
                Some(value) => value,
            };
            let datos_votantes =
                match sistema_elecciones.obtener_votantes_eleccion_por_id(id_eleccion) {
                    Err(msg) => return Err(msg),
                    Ok(datos) => datos,
                };

            let cantidad_votantes = datos_votantes.len() as u32;
            let cantidad_votantes_voto_efectivo = datos_votantes
                .iter()
                .filter(|vot: &&(ink::primitives::AccountId, bool)| vot.1)
                .count() as u32;
            let porcentaje_participacion = cantidad_votantes_voto_efectivo
                .mul(100)
                .div_ceil(cantidad_votantes);
            Ok((cantidad_votantes_voto_efectivo, porcentaje_participacion))
        }

        /// Permite obtener un reporte los datos de un candidato en particular dentro de una elección específica.
        /// Parámetros
        ///    eleccion_id (u64): El ID de la elección de la cual se quiere obtener la información del candidato.
        ///
        /// Retorno
        /// Result< ((AccountId, String, String, String, u32), Vec<(AccountId, String, String, String, u32)>), String>:
        /// Los datos del ganador de la eleccion si no resulta en empate y un Vector ordenado con: ID de cada candidato, Nombre, Apellido, DNI y su total de votos,
        /// o un mensaje de error
        #[ink(message)]
        pub fn reporte_de_resultado_por_eleccion(
            &mut self,
            id_eleccion: u64,
        ) -> Result<
            (
                Option<(AccountId, String, String, String, u32)>,
                Vec<(AccountId, String, String, String, u32)>,
            ),
            String,
        > {
            let sistema_elecciones = match &mut self.sistema_elecciones {
                None => return Err("Sistema elecciones no seteado".to_string()),
                Some(value) => value,
            };
            let mut datos_candidatos =
                match sistema_elecciones.obtener_candidatos_eleccion_por_id(id_eleccion) {
                    Err(msg) => return Err(msg),
                    Ok(datos) => datos,
                };

            // Ordenar datos_candidatos por la cantidad de votos (descendente)
            datos_candidatos.sort_by(|a, b| b.1.cmp(&a.1));

            let candidatos: Vec<(ink::primitives::AccountId, String, String, String, u32)> =
                datos_candidatos
                    .iter()
                    .map(|datos_candidato| {
                        let datos_usuario = sistema_elecciones
                            .obtener_informacion_usuario(datos_candidato.0)
                            .unwrap_or_default();
                        (
                            datos_candidato.0.clone(),
                            datos_usuario.0,
                            datos_usuario.1,
                            datos_usuario.2,
                            datos_candidato.1,
                        )
                    })
                    .collect();

            if candidatos.len() >= 2 && candidatos[0].4 == candidatos[1].4 {
                return Ok((None, candidatos));
            }
            Ok((Some(candidatos[0].clone()), candidatos))
        }
    }

    // #[cfg(test)]
    struct SistemaEleccionesFake;

    // #[cfg(test)]
    impl SistemaEleccionesFake {
        fn obtener_votantes_eleccion_por_id(
            &self,
            id_eleccion: u32,
        ) -> Result<Vec<(AccountId, bool)>, String> {
            match id_eleccion {
                1 => Ok(vec![
                    (AccountId::from([0x08; 32]), true),
                    (AccountId::from([0x01; 32]), true),
                    (AccountId::from([0x05; 32]), false),
                    (AccountId::from([0x07; 32]), false),
                    (AccountId::from([0x03; 32]), true),
                    (AccountId::from([0x09; 32]), true),
                    (AccountId::from([0x02; 32]), true),
                ]),
                2 => Ok(vec![
                    (AccountId::from([0x08; 32]), true),
                    (AccountId::from([0x07; 32]), true),
                    (AccountId::from([0x09; 32]), false),
                    (AccountId::from([0x02; 32]), true),
                    (AccountId::from([0x06; 32]), true),
                    (AccountId::from([0x05; 32]), true),
                    (AccountId::from([0x03; 32]), true),
                    (AccountId::from([0x04; 32]), true),
                    (AccountId::from([0x01; 32]), true),
                ]),
                3 => Ok(vec![
                    (AccountId::from([0x01; 32]), true),
                    (AccountId::from([0x08; 32]), true),
                    (AccountId::from([0x02; 32]), false),
                    (AccountId::from([0x06; 32]), true),
                    (AccountId::from([0x04; 32]), true),
                ]),
                _ => Err("Eleccion no existe".to_string()),
            }
        }

        fn obtener_candidatos_eleccion_por_id(
            &self,
            id_eleccion: u32,
        ) -> Result<Vec<(AccountId, u32)>, String> {
            match id_eleccion {
                1 => Ok(vec![
                    (AccountId::from([0x0A; 32]), 2),
                    (AccountId::from([0x0C; 32]), 3),
                ]),
                2 => Ok(vec![
                    (AccountId::from([0x0B; 32]), 1),
                    (AccountId::from([0x0A; 32]), 5),
                    (AccountId::from([0x0C; 32]), 2),
                ]),
                3 => Ok(vec![
                    (AccountId::from([0x0A; 32]), 2),
                    (AccountId::from([0x0C; 32]), 2),
                ]),
                _ => Err("Eleccion no existe".to_string()),
            }
        }

        fn obtener_informacion_usuario(
            &self,
            id_usuario: AccountId,
        ) -> Option<(String, String, String)> {
            if id_usuario == AccountId::from([0x01; 32]) {
                Some((
                    "Alice".to_string(),
                    "Wonderland".to_string(),
                    "54326961".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x02; 32]) {
                Some((
                    "Bob".to_string(),
                    "Builder".to_string(),
                    "64128970".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x03; 32]) {
                Some((
                    "Carlos".to_string(),
                    "Caceres".to_string(),
                    "54326961".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x04; 32]) {
                Some((
                    "Ana".to_string(),
                    "Martínez".to_string(),
                    "45678901".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x05; 32]) {
                Some((
                    "Luis".to_string(),
                    "Sánchez".to_string(),
                    "56789012".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x06; 32]) {
                Some((
                    "Elena".to_string(),
                    "Rodríguez".to_string(),
                    "67890123".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x07; 32]) {
                Some((
                    "Pedro".to_string(),
                    "Fernández".to_string(),
                    "78901234".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x08; 32]) {
                Some((
                    "Juan".to_string(),
                    "Pérez".to_string(),
                    "12345678".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x09; 32]) {
                Some((
                    "María".to_string(),
                    "González".to_string(),
                    "23456789".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x0A; 32]) {
                //CANDIDATO
                Some((
                    "Carlos".to_string(),
                    "Gómez".to_string(),
                    "34567890".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x0B; 32]) {
                //CANDIDATO
                Some((
                    "Ricardo".to_string(),
                    "Palacios".to_string(),
                    "24218796".to_string(),
                ))
            } else if id_usuario == AccountId::from([0x0C; 32]) {
                //CANDIDATO
                Some((
                    "Tomas".to_string(),
                    "Lopez".to_string(),
                    "78921353".to_string(),
                ))
            } else {
                None
            }
        }
    }

    // #[cfg(test)]
    struct ReporteFake {
        sistema_elecciones: Option<SistemaEleccionesFake>,
    }

    // #[cfg(test)]
    impl ReporteFake {
        fn new(sistema_elecciones: SistemaEleccionesFake) -> Self {
            ReporteFake {
                sistema_elecciones: Some(sistema_elecciones),
            }
        }

        fn new_vacio() -> Self {
            ReporteFake {
                sistema_elecciones: None,
            }
        }

        fn reporte_de_votantes_por_eleccion(
            &mut self,
            id_eleccion: u32,
        ) -> Result<Vec<(AccountId, String, String, String)>, String> {
            let sistema_elecciones = match &mut self.sistema_elecciones {
                None => return Err("Sistema elecciones no seteado".to_string()),
                Some(value) => value,
            };
            let datos_votantes =
                match sistema_elecciones.obtener_votantes_eleccion_por_id(id_eleccion) {
                    Err(msg) => return Err(msg),
                    Ok(datos) => datos,
                };
            let reporte = datos_votantes
                .iter()
                .map(|datos_votante| {
                    let datos_usuario = sistema_elecciones
                        .obtener_informacion_usuario(datos_votante.0)
                        .unwrap_or_default();
                    (
                        datos_votante.0.clone(),
                        datos_usuario.0,
                        datos_usuario.1,
                        datos_usuario.2,
                    )
                })
                .collect();
            Ok(reporte)
        }

        fn reporte_de_participacion_por_eleccion(
            &mut self,
            id_eleccion: u32,
        ) -> Result<(u32, u32), String> {
            let sistema_elecciones = match &mut self.sistema_elecciones {
                None => return Err("Sistema elecciones no seteado".to_string()),
                Some(value) => value,
            };
            let datos_votantes =
                match sistema_elecciones.obtener_votantes_eleccion_por_id(id_eleccion) {
                    Err(msg) => return Err(msg),
                    Ok(datos) => datos,
                };

            let cantidad_votantes = datos_votantes.len() as u32;
            let cantidad_votantes_voto_efectivo = datos_votantes
                .iter()
                .filter(|vot: &&(ink::primitives::AccountId, bool)| vot.1)
                .count() as u32;
            let porcentaje_participacion = cantidad_votantes_voto_efectivo
                .mul(100)
                .div_ceil(cantidad_votantes);
            Ok((cantidad_votantes_voto_efectivo, porcentaje_participacion))
        }

        fn reporte_de_resultado_por_eleccion(
            &mut self,
            id_eleccion: u32,
        ) -> Result<
            (
                Option<(AccountId, String, String, String, u32)>,
                Vec<(AccountId, String, String, String, u32)>,
            ),
            String,
        > {
            let sistema_elecciones = match &mut self.sistema_elecciones {
                None => return Err("Sistema elecciones no seteado".to_string()),
                Some(value) => value,
            };
            let mut datos_candidatos =
                match sistema_elecciones.obtener_candidatos_eleccion_por_id(id_eleccion) {
                    Err(msg) => return Err(msg),
                    Ok(datos) => datos,
                };

            // Ordenar datos_candidatos por la cantidad de votos (descendente)
            datos_candidatos.sort_by(|a, b| b.1.cmp(&a.1));

            let candidatos: Vec<(ink::primitives::AccountId, String, String, String, u32)> =
                datos_candidatos
                    .iter()
                    .map(|datos_candidato| {
                        let datos_usuario = sistema_elecciones
                            .obtener_informacion_usuario(datos_candidato.0)
                            .unwrap_or_default();
                        (
                            datos_candidato.0.clone(),
                            datos_usuario.0,
                            datos_usuario.1,
                            datos_usuario.2,
                            datos_candidato.1,
                        )
                    })
                    .collect();

            if candidatos.len() >= 2 && candidatos[0].4 == candidatos[1].4 {
                return Ok((None, candidatos));
            }
            Ok((Some(candidatos[0].clone()), candidatos))
        }
    }
    // Módulo de pruebas
    #[cfg(test)]
    mod tests {
        use ink::primitives::AccountId;

        use super::ReporteFake;
        use super::SistemaEleccionesFake;

        #[test]
        fn test_reporte_de_votantes_por_eleccion_error_sin_sistema() {
            let mut reporte = ReporteFake::new_vacio();
            let result = reporte.reporte_de_votantes_por_eleccion(0);
            assert!(result.is_err());
        }

        #[test]
        fn test_reporte_de_votantes_por_eleccion_error_no_eleccion() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);
            let result = reporte.reporte_de_votantes_por_eleccion(0);
            assert!(result.is_err());
        }

        #[test]
        fn test_reporte_de_votantes_por_eleccion_exito() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);
            let result = reporte.reporte_de_votantes_por_eleccion(1);

            assert!(result.is_ok());
            assert_eq!(result.as_ref().unwrap().len(), 7);

            assert_eq!(result.as_ref().unwrap()[2].0, AccountId::from([0x05; 32]));
            assert_ne!(result.as_ref().unwrap()[2].1, "Alice".to_string());

            assert_eq!(result.as_ref().unwrap()[4].0, AccountId::from([0x03; 32]));
            assert_eq!(result.as_ref().unwrap()[4].1, "Carlos".to_string());

            let result = reporte.reporte_de_votantes_por_eleccion(3);

            assert!(result.is_ok());
            assert_eq!(result.as_ref().unwrap().len(), 5);

            assert_eq!(result.as_ref().unwrap()[3].0, AccountId::from([0x06; 32]));
            assert_eq!(result.as_ref().unwrap()[4].0, AccountId::from([0x04; 32]));
        }

        #[test]
        fn test_reporte_de_participacion_por_eleccion_error_sin_sistema() {
            let mut reporte = ReporteFake::new_vacio();
            let result = reporte.reporte_de_participacion_por_eleccion(0);
            assert!(result.is_err());
        }
        #[test]
        fn test_reporte_de_participacion_por_eleccion_error_no_eleccion() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);
            let result = reporte.reporte_de_participacion_por_eleccion(0);
            assert!(result.is_err());
        }

        #[test]
        fn test_reporte_de_participacion_por_eleccion_exito_1() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);

            let result = reporte.reporte_de_participacion_por_eleccion(3);
            assert!(result.is_ok());
            assert_eq!(result.as_ref().unwrap().0, 4);
            assert_eq!(result.as_ref().unwrap().1, 80);

            let result = reporte.reporte_de_participacion_por_eleccion(2);
            assert!(result.is_ok());
            assert_eq!(result.as_ref().unwrap().0, 8);
            assert_eq!(result.as_ref().unwrap().1, 89);

            let result = reporte.reporte_de_participacion_por_eleccion(1);
            assert!(result.is_ok());
            assert_eq!(result.as_ref().unwrap().0, 5);
            assert_eq!(result.as_ref().unwrap().1, 72);
        }

        #[test]
        fn test_reporte_de_resultado_por_eleccion_error_sin_sistema() {
            let mut reporte = ReporteFake::new_vacio();
            let result = reporte.reporte_de_resultado_por_eleccion(0);
            assert!(result.is_err());
        }
        #[test]
        fn test_reporte_de_resultado_por_eleccion_error_no_eleccion() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);
            let result = reporte.reporte_de_resultado_por_eleccion(0);
            assert!(result.is_err());
        }

        #[test]
        fn test_reporte_de_resultado_por_eleccion_exito_empate() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);
            let result = reporte.reporte_de_resultado_por_eleccion(3);

            assert!(result.is_ok());
            assert!(result.as_ref().unwrap().0.is_none());

            assert_eq!(result.as_ref().unwrap().1[0].0, AccountId::from([0x0A; 32]));
            assert_eq!(result.as_ref().unwrap().1[0].4, 2);

            assert_eq!(result.as_ref().unwrap().1[1].0, AccountId::from([0x0C; 32]));
            assert_eq!(result.as_ref().unwrap().1[1].4, 2);
        }

        #[test]
        fn test_reporte_de_resultado_por_eleccion_exito_victoria_1() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);
            let result = reporte.reporte_de_resultado_por_eleccion(2);

            assert!(result.is_ok());
            assert!(result.as_ref().unwrap().0.is_some());

            assert_ne!(result.as_ref().unwrap().1[0].0, AccountId::from([0x0B; 32]));
            // (AccountId::from([0x0B; 32]), 1),
            // (AccountId::from([0x0A; 32]), 5),
            // (AccountId::from([0x0C; 32]), 2),

            assert_eq!(result.as_ref().unwrap().1[0].0, AccountId::from([0x0A; 32]));
            assert_eq!(result.as_ref().unwrap().1[0].4, 5);

            assert_eq!(result.as_ref().unwrap().1[1].0, AccountId::from([0x0C; 32]));
            assert_eq!(result.as_ref().unwrap().1[1].4, 2);

            assert_eq!(result.as_ref().unwrap().1[2].0, AccountId::from([0x0B; 32]));
            assert_eq!(result.as_ref().unwrap().1[2].4, 1);
        }
        #[test]
        fn test_reporte_de_resultado_por_eleccion_exito_victoria_2() {
            let sist_elecciones = SistemaEleccionesFake;
            let mut reporte = ReporteFake::new(sist_elecciones);
            let result = reporte.reporte_de_resultado_por_eleccion(1);

            assert!(result.is_ok());
            assert!(result.as_ref().unwrap().0.is_some());

            assert_ne!(result.as_ref().unwrap().1[0].0, AccountId::from([0x0B; 32]));

            // (AccountId::from([0x0A; 32]), 2),
            // (AccountId::from([0x0C; 32]), 3),

            assert_eq!(result.as_ref().unwrap().1[0].0, AccountId::from([0x0C; 32]));
            assert_eq!(result.as_ref().unwrap().1[0].4, 3);

            assert_eq!(result.as_ref().unwrap().1[1].0, AccountId::from([0x0A; 32]));
            assert_eq!(result.as_ref().unwrap().1[1].4, 2);
        }
    }
}

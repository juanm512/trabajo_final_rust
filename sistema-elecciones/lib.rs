#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use self::sistema_elecciones::{SistemaElecciones, SistemaEleccionesRef};

#[ink::contract]
mod sistema_elecciones {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use scale_info::prelude::format;

    enum ERRORES
    {
        NO_ES_ADMINISTRADOR,
        USUARIO_NO_REGISTRADO,
    }

    #[derive(scale::Decode, scale::Encode, Debug, PartialEq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout,Clone))]
    pub enum TIPO_DE_USUARIO
    {
        VOTANTE,
        CANDIDATO
    }

    impl ERRORES
    {
        fn to_string(&self) -> String
        {
            match self 
            {
                ERRORES::NO_ES_ADMINISTRADOR => String::from("No eres el administrador."),
                ERRORES::USUARIO_NO_REGISTRADO => String::from("No estás registrado en el sistema. Espera a que te acepten en el mismo o realiza la solicitud.")
            }
        }
    }

    #[derive(scale::Decode, scale::Encode, Debug,Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct Usuario
    {
        id:AccountId,
        nombre:String,
        apellido:String,
        dni:String,
    }

    #[derive(scale::Decode, scale::Encode, Debug,Clone,PartialEq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Votante
    {
        id:AccountId,
        voto_emitido:bool,
    }

    #[derive(scale::Decode, scale::Encode, Debug,Clone,PartialEq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct CandidatoConteo
    {
        id:AccountId,
        candidato_id:u32,
        votos_totales:u32,
    }

    #[derive(scale::Decode, scale::Encode, Debug,PartialEq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct Eleccion
    {
        id:u64,
        candidatos:Vec<CandidatoConteo>,
        votantes:Vec<Votante>,
        usuarios_rechazados:Vec<AccountId>,
        usuarios_pendientes:Vec<(AccountId,TIPO_DE_USUARIO)>,
        votacion_iniciada:bool,
        fecha_inicio:u64,
        fecha_final:u64,
        resultados:Option<Resultados>
    }

    #[derive(scale::Decode, scale::Encode, Debug, Clone,PartialEq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Resultados
    {
        votos_totales:u64, // Votos totales, cuentan los que votaron y no votaron
        votos_realizados:u64, // Votos realizados, cuentan solo los que votaron
        votos_candidatos:Vec<(AccountId, u64)>
    }

    impl Eleccion
    {
        /// Este método verifica si hay un usuario pendiente con el ID especificado en la lista de usuarios pendientes.
        fn contiene_usuario_pendiente(&self, id: AccountId) -> bool {
            self.usuarios_pendientes.iter().any(|(usuario_id, _tipo)| *usuario_id == id)
        }

        /// Este método verifica si existe un candidato con el ID especificado dentro de la lista de candidatos disponibles.
        fn existe_candidato(&self, candidato_id:u32) -> bool
        {
            candidato_id >= 1 && candidato_id <= self.candidatos.len() as u32
        }
        //Verifica que exista el candidato del que se esta intentando obtener la informacion
        //En el caso de que exista retorna el candidato pasado por parametro
        fn obtener_informacion_candidato(&self, candidato_id:u32) -> Option<&CandidatoConteo>
        {
            if !self.existe_candidato(candidato_id) { return None; }
            match (candidato_id as usize).checked_sub(1) {
                None => None,
                Some(index) => Some(&self.candidatos[index])
            }
        }
        /// Permite a un votante registrado emitir su voto por un candidato en la elección actual.
        /// - votante_id: AccountId del votante que desea votar.
        /// - candidato_id: Identificador único del candidato por el cual se desea emitir el voto.
        /// Descripción:
        /// Verifica si el candidato existe. Si no existe, retorna un error.
        /// Verifica si el votante está registrado en la elección. Si no lo está, retorna un error.
        /// Verifica si el votante ya ha emitido su voto. Si ya lo hizo, retorna un error.
        /// Registra el voto del votante por el candidato especificado y actualiza los conteos.
        /// Retorna un mensaje de éxito si el voto se emitió correctamente.
        pub fn votar_candidato(&mut self, votante_id:AccountId, candidato_id:u32) -> Result<String, String>
        {
            if !self.existe_candidato(candidato_id) { return Err(String::from("No existe un candidato con este id.")); }

            let votante = match self.votantes.iter_mut().find(|votante| votante.id == votante_id) {
                Some(votante) => votante,
                None => return Err(String::from("No estás registrado en la elección."))
            };
            if votante.voto_emitido { return Err(String::from("No se realizó el voto porque ya votaste anteriormente.")); }
            votante.voto_emitido = true;

            let candidato = match (candidato_id as usize).checked_sub(1) {
                None => return Err(String::from("Se produjo un overflow intentando obtener el candidato.")),
                Some(index) => &mut self.candidatos[index]
            };
            match candidato.votos_totales.checked_add(1) {
                None => {
                    votante.voto_emitido = false;
                    return Err(String::from("Se produjo un overflow al intentar sumar el voto."));
                },
                Some(votos_totales) => {
                    candidato.votos_totales = votos_totales;
                    return Ok(String::from("Voto emitido exitosamente."));
                }
            }
        }

        /// Usado por el administrador.
        /// Procesa el siguiente usuario pendiente, aceptándolo o rechazándolo según el parámetro aceptar_usuario.
        /// - aceptar_usuario: bool - Indica si se acepta (true) o se rechaza (false) al usuario pendiente.
        /// Result<String, String>: Mensaje de éxito indicando si el usuario fue agregado o rechazado correctamente, o un mensaje de error si no hay usuarios pendientes.
        /// Descripción:
        /// Verifica si hay usuarios pendientes. Si no hay, retorna un error indicando que no hay usuarios pendientes.
        /// Si se acepta al usuario, lo añade a la lista correspondiente (votantes o candidatos) según su tipo. Retorna un mensaje de éxito indicando la operación realizada.
        /// Si se rechaza al usuario, lo añade a la lista de usuarios rechazados. Retorna un mensaje de éxito indicando que el usuario fue rechazado.

        pub fn procesar_siguiente_usuario_pendiente(&mut self, aceptar_usuario:bool) -> Result<String, String>
        {
            let sig_usuario = self.usuarios_pendientes.first();
            if sig_usuario.is_none() { return Err(String::from("No hay usuarios pendientes.")); }

            let (usuario, tipo) = self.usuarios_pendientes.remove(0);
            if aceptar_usuario { 
                match tipo {
                    TIPO_DE_USUARIO::VOTANTE =>{
                    self.votantes.push(Votante{
                        id:usuario,
                        voto_emitido:false,
                    });

                   },
                   TIPO_DE_USUARIO::CANDIDATO=>{

                    let candidato_id = match (self.candidatos.len() as u32).checked_add(1) {
                        Some(id_validado) => id_validado,
                        None => return Err(String::from("Ocurrio un overflow al calcular la ID del candidato.")),
                    };
                    self.candidatos.push(CandidatoConteo{
                        id:usuario,
                        candidato_id,
                        votos_totales:0,
                    });
                   },
                }
                return Ok(String::from("Usuario agregado exitosamente."));
            }
            else{
                self.usuarios_rechazados.push(usuario);
                return Ok(String::from("Usuario rechazado exitosamente."));
            }
        }
        /// Obtiene los resultados de la votación si están disponibles.
        /// - block_timestamp: u64 - Marca de tiempo del bloque para verificar si ya se pueden obtener los resultados.
        /// Retorno:
        /// Option<&Resultados>: Resultados de la votación si están disponibles, o None si no lo están.
        /// Descripción:
        /// Verifica si la fecha final de la elección es mayor que el block_timestamp. Si es así, retorna None ya que los resultados aún no están disponibles.
        /// Si self.resultados tiene algún valor, devuelve una referencia a esos resultados.
        /// Si no hay resultados previos, calcula los resultados contando los votos totales y realizados de los votantes, y los votos totales de cada candidato.
 
        fn obtener_resultados_votacion(&mut self, block_timestamp:u64) -> Option<&Resultados>
        {
            if self.fecha_final > block_timestamp {
                return None;
            }

            if self.resultados.is_some() {
                return self.resultados.as_ref();
            }

            let mut resultados = Resultados { 
                votos_totales: 0, 
                votos_realizados: 0,
                votos_candidatos: Vec::new(),
            };

            resultados.votos_totales = self.votantes.len() as u64;
            resultados.votos_realizados = self.votantes.iter().filter(|v| v.voto_emitido).count() as u64;
            self.candidatos.iter().for_each(|c| {
                resultados.votos_candidatos.push((c.id, c.votos_totales as u64));
            });

            self.resultados = Some(resultados);
            return self.resultados.as_ref();
        }
    }

    #[ink(storage)]
    pub struct SistemaElecciones {
        administrador:AccountId,
        generador_reportes:Option<AccountId>,
        registro_activado:bool,
        usuarios:Vec<Usuario>,
        usuarios_pendientes:Vec<Usuario>,
        usuarios_rechazados:Vec<AccountId>,
        elecciones:Vec<Eleccion>,
    }

    #[ink(impl)]
    impl SistemaElecciones {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self { 
                administrador: Self::env().caller(),
                generador_reportes: None,
                registro_activado: false,
                usuarios: Vec::new(),
                usuarios_pendientes: Vec::new(),
                usuarios_rechazados: Vec::new(),
                elecciones: Vec::new(),
            }
        }
        //Verifica que el caller sea el generador de reportes
        fn es_generador_reportes(&self) -> bool
        {
            match self.generador_reportes { 
                None => false,
                Some(val) => self.env().caller() == val
            }
        }
        //Verifica que el caller sea el administrador del sistema
        fn es_administrador(&self) -> bool
        {
            self.env().caller() == self.administrador
        }
        //Dado un id retorna Option de un usuario
        fn obtener_usuario(&self, id: AccountId) -> Option<&Usuario> 
        {
            self.usuarios.iter().find(|usuario| usuario.id == id)
        }
        //Verifica que el adress del que hace la llamada al contrato sea un usuario registrado
        fn es_usuario_registrado(&self) -> bool
        {
            let id = self.env().caller();
            self.usuarios.iter().any(|usuario| usuario.id == id)
        }
        //Verifica que el adress del que hace la llamada al contrato sea un usuario pendiente
        fn es_usuario_pendiente(&self) -> bool
        {
            let id=self.env().caller();
            self.usuarios_pendientes.iter().any(|usuario| usuario.id == id)
        }
        //Verifica que la eleccion pasa por parametro exista y retorna un booleano
        fn existe_eleccion(&self, eleccion_id:u64) -> bool
        {
            eleccion_id >= 1 && eleccion_id <= self.elecciones.len() as u64
        }

        /// Obtiene una referencia mutable a una elección por su ID.
        /// - eleccion_id: u64 - ID de la elección a obtener.
        /// Option<&mut Eleccion>: Referencia mutable a la elección si existe, o None si no se encuentra.
        /// Descripción:
        /// Verifica si existe una elección con el ID proporcionado utilizando el método existe_eleccion().
        /// Si la elección existe, calcula el índice válido (ajustado para el acceso a vector) y retorna una referencia mutable a la elección.
        /// Retorna None si no se encuentra ninguna elección con el ID especificado.
        fn obtener_eleccion_por_id(&mut self, eleccion_id:u64) -> Option<&mut Eleccion>
        {
            if self.existe_eleccion(eleccion_id) {
                match eleccion_id.checked_sub(1) {
                    Some(index_valid) => return Some(&mut self.elecciones[index_valid as usize]),
                    None => return None
                }
            }
            return None;
        }
        /// Obtiene una referencia inmutable a una elección por su ID.
        /// - eleccion_id: u64 - ID de la elección a obtener.
        /// Option<&Eleccion>: Referencia inmutable a la elección si existe, o None si no se encuentra.
        /// Descripción:
        /// Verifica si existe una elección con el ID proporcionado utilizando el método existe_eleccion().
        /// Si la elección existe, calcula el índice válido (ajustado para el acceso a vector) y retorna una referencia inmutable a la elección.
        /// Retorna None si no se encuentra ninguna elección con el ID especificado.
        fn obtener_ref_eleccion_por_id(&self, eleccion_id:u64) -> Option<&Eleccion>
        {
            if self.existe_eleccion(eleccion_id) {
                match eleccion_id.checked_sub(1) {
                    Some(index_valid) => return Some(&self.elecciones[index_valid as usize]),
                    None => return None
                }
            }
            return None;
        }
        /// Valida el estado de una elección antes de permitir que un usuario se registre en ella.
        /// - eleccion_id: u64 - ID de la elección a validar.
        /// - block_timestamp: u64 - Timestamp del bloque actual.
        /// - id_usuario: AccountId - ID del usuario que desea registrarse en la elección.
        /// Result<&mut Eleccion, String>: Referencia mutable a la elección válida o mensaje de error.
        /// Descripción:
        /// Verifica si existe una elección con el ID proporcionado. Si no existe, retorna un error indicando que no existe una elección con ese ID.
        /// Obtiene la elección y verifica si el usuario ya está registrado como pendiente en ella. Si lo está, retorna un error indicando que ya está registrado en la elección.
        /// Verifica si la votación en la elección ya ha iniciado o si la fecha de inicio es menor al timestamp del bloque actual. Si es así, retorna un error indicando que la votación ya comenzó y no se puede registrar.
        /// Verifica si la fecha de finalización de la elección es menor al timestamp del bloque actual. Si es así, retorna un error indicando que la elección ya finalizó y no se puede registrar.
        /// Retorna una referencia mutable a la elección válida si todas las validaciones pasan.
        fn validar_estado_eleccion(&mut self,eleccion_id:u64,block_timestamp:u64,id_usuario:AccountId) -> Result<&mut Eleccion,String>
        {
            let option_eleccion = self.obtener_eleccion_por_id(eleccion_id);
            if option_eleccion.is_none() { return Err(String::from("No existe una elección con ese id.")); }
            
            let eleccion = option_eleccion.unwrap();
            if eleccion.contiene_usuario_pendiente(id_usuario) { return Err(String::from("Ya está registrado en la elección.")); }
            
            if eleccion.votacion_iniciada || eleccion.fecha_inicio < block_timestamp {
                return Err(String::from("La votación en la elección ya comenzó, no te puedes registrar."));
            }
            if eleccion.fecha_final < block_timestamp {
                return Err(String::from("La elección ya finalizó, no te puedes registrar."));
            }
            Ok(eleccion)
        }

        // ====-----==== METODOS PÚBLICOS ====----====
        // ====-----==== METODOS PÚBLICOS ====----====
        // ====-----==== METODOS PÚBLICOS ====----====

    
        /// Registra un nuevo usuario en el sistema.
        /// - nombre: String - Nombre del usuario a registrar.
        /// - apellido: String - Apellido del usuario a registrar.
        /// - dni: String - DNI del usuario a registrar.
        /// Descripción:
        /// Verifica si el registro está activado. Si no lo está, retorna un error indicando que el registro no está activado.
        /// Verifica si el llamador es el administrador. Si es así, retorna un error indicando que el administrador no puede registrarse.
        /// Verifica si el llamador ya ha sido rechazado anteriormente. Si es así, retorna un error indicando que la solicitud de registro ya fue rechazada.
        /// Verifica si el llamador ya está registrado como usuario. Si es así, retorna un error indicando que ya está registrado.
        /// Verifica si el llamador ya está en la cola de usuarios pendientes. Si es así, retorna un error indicando que ya está en la cola de usuarios pendientes.
        /// Registra al usuario añadiéndolo a la cola de usuarios pendientes y retorna un mensaje de éxito.
        #[ink(message)]
        pub fn registrarse(&mut self, nombre:String, apellido:String, dni:String) -> Result<String, String>
        {
            self.registrarse_privado(nombre, apellido, dni)
        }
        fn registrarse_privado(&mut self, nombre:String, apellido:String, dni:String) -> Result<String, String>
        {
            if !self.registro_activado { return Err(String::from("El registro todavía no está activado.")); }
            let id = self.env().caller();
            if self.es_administrador() { return Err(String::from("Eres el administrador, no puedes registrarte.")); }
            if self.usuarios_rechazados.contains(&id) { 
                return Err(String::from("Tu solicitud de registro ya fue rechazada."));
            }
            if self.es_usuario_registrado()
            {
                return Err(String::from("Ya estás registrado como usuario."));
            }
            if self.es_usuario_pendiente()
            {
                return Err(String::from("Ya estás en la cola de usuarios pendientes."));    
            }
            let usuario = Usuario { id, nombre, apellido, dni };
            self.usuarios_pendientes.push(usuario);
            return Ok(String::from("Registro exitoso. Se te añadió en la cola de usuarios pendientes."));
        }
        
        // ====-----==== METODOS ADMINISTRADOR ====----====

        /// Obtiene la información del siguiente usuario pendiente en la lista.
        /// Result<String, String>: Mensaje de éxito con la información del usuario o error si no hay usuarios pendientes.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Obtiene al siguiente usuario pendiente en la lista de usuarios pendientes.
        /// Retorna un mensaje de éxito con el nombre, apellido y DNI del usuario pendiente si existe.
        /// Si no hay usuarios pendientes, retorna un error indicando que no hay usuarios pendientes.
        #[ink(message)]
        pub fn obtener_informacion_siguiente_usuario_pendiente(&self) -> Result<String, String>
        {
            self.obtener_informacion_siguiente_usuario_pendiente_privado()
        }
        fn obtener_informacion_siguiente_usuario_pendiente_privado(&self) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let sig_usuario = self.usuarios_pendientes.first();
            match sig_usuario {
                Some(usuario) => {
                    let mut str = String::from("Nombre: ") + usuario.nombre.as_str();
                    str.push_str((String::from("\nApellido: ") + usuario.apellido.as_str()).as_str());
                    str.push_str((String::from("\nDNI: ") + usuario.dni.as_str()).as_str());
                    return Ok(str);
                },
                None => Err(String::from("No hay usuarios pendientes.")),
            }
        }
        /// Procesa el siguiente usuario pendiente, aceptándolo o rechazándolo.
        /// - aceptar_usuario: bool - Indica si se acepta (true) o se rechaza (false) al usuario pendiente.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Obtiene al siguiente usuario pendiente en la lista. Si no hay usuarios pendientes, retorna un error.
        /// Si se acepta al usuario, lo mueve desde la lista de usuarios pendientes a la lista de usuarios.
        /// Si se rechaza al usuario, lo agrega a la lista de usuarios rechazados.
        /// Retorna un mensaje de éxito indicando si el usuario fue agregado o rechazado exitosamente.
        #[ink(message)]
        pub fn procesar_siguiente_usuario_pendiente(&mut self, aceptar_usuario:bool) -> Result<String, String>
        {
            self.procesar_siguiente_usuario_pendiente_privado(aceptar_usuario)
        }
        fn procesar_siguiente_usuario_pendiente_privado(&mut self, aceptar_usuario:bool) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let sig_usuario = self.usuarios_pendientes.first();
            if sig_usuario.is_none() { return Err(String::from("No hay usuarios pendientes.")); }
    
            let usuario = self.usuarios_pendientes.remove(0);
            if aceptar_usuario { 
                self.usuarios.push(usuario);
                return Ok(String::from("Usuario agregado exitosamente."));
            }
    
            self.usuarios_rechazados.push(usuario.id);
            return Ok(String::from("Usuario rechazado exitosamente."));
        }


        // ====-----==== METODOS ELECCIONES ====----====
        // ====-----==== METODOS ELECCIONES ====----====
        // ====-----==== METODOS ELECCIONES ====----====

        // ====-----==== METODOS ADMINISTRADOR ====----====

        /// Crea una nueva elección con las fechas especificadas.
        /// - fecha_inicial: String - Fecha y hora de inicio en formato "dd-mm-YYYY hh:mm".
        /// - fecha_final: String - Fecha y hora de finalización en formato "dd-mm-YYYY hh:mm".
        /// Result<String, String>: Mensaje de éxito con el ID de la elección creada o mensaje de error.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Parsea las fechas iniciales y finales proporcionadas en formato específico. Retorna un error si el formato no es válido.
        /// Genera un nuevo ID para la elección basado en el tamaño actual de la lista de elecciones.
        /// Crea una nueva instancia de `Eleccion` con los parámetros dados y la agrega a la lista de elecciones.
        /// Retorna un mensaje de éxito indicando que la elección fue creada exitosamente junto con el ID de la elección.
        #[ink(message)]
        pub fn crear_eleccion(&mut self, fecha_inicial:String, fecha_final:String) -> Result<String, String>
        {
            self.crear_eleccion_privado(fecha_inicial, fecha_final)
        }
        fn crear_eleccion_privado(&mut self, fecha_inicial: String, fecha_final: String) -> Result<String, String> 
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
    
            let fecha_inicial_milisegundos = chrono::NaiveDateTime::parse_from_str(&fecha_inicial, "%d-%m-%Y %H:%M");
            if fecha_inicial_milisegundos.is_err() {
                return Err(String::from("Error en el formato de la fecha inicial. Formato: dd-mm-YYYY hh:mm"));
            }
            let fecha_final_milisegundos = chrono::NaiveDateTime::parse_from_str(&fecha_final, "%d-%m-%Y %H:%M");
            if fecha_final_milisegundos.is_err() {
                return Err(String::from("Error en el formato de la fecha final. Formato: dd-mm-YYYY hh:mm"));
            }
    
            let eleccion_id = match (self.elecciones.len() as u64).checked_add(1) {
                Some(index) => index,
                None => return Err(String::from("Se produjo un overflow al intentar crear una elección."))
            };
            let eleccion = Eleccion {
                id: eleccion_id,
                candidatos: Vec::new(),
                votantes: Vec::new(),
                usuarios_pendientes: Vec::new(),
                usuarios_rechazados: Vec::new(),
                votacion_iniciada:false,
                fecha_inicio: fecha_inicial_milisegundos.unwrap().and_utc().timestamp_millis() as u64,
                fecha_final: fecha_final_milisegundos.unwrap().and_utc().timestamp_millis() as u64,
                resultados:None
            };
            self.elecciones.push(eleccion);
    
            return Ok(format!("Eleccion creada exitosamente. Id de la elección: {}", eleccion_id));
        }

        /// Permite al administrador iniciar una votación si la fecha actual es posterior a la fecha inicial establecida y la votación aún no ha comenzado.
        /// Inicia la votación de una elección.
        /// - eleccion_id: u64 - ID de la elección que se desea iniciar.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Obtiene la elección correspondiente al ID proporcionado y verifica las condiciones para iniciar la votación:
        /// - Si el tiempo actual es posterior a la fecha de finalización de la elección, retorna un error indicando que la votación ya finalizó.
        /// - Si la votación ya está iniciada, retorna un error indicando que la votación ya inició.
        /// - Si el tiempo actual es anterior a la fecha de inicio de la elección, retorna un error indicando que todavía no es la fecha para la votación.
        /// Si todas las condiciones son satisfactorias, marca la votación como iniciada y retorna un mensaje de éxito.
        #[ink(message)]
        pub fn iniciar_votacion(&mut self, eleccion_id:u64) -> Result<String, String>
        {
            self.iniciar_votacion_privado(eleccion_id)
        }
        pub fn iniciar_votacion_privado(&mut self, eleccion_id:u64) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let block_timestamp = self.env().block_timestamp();

            match self.obtener_eleccion_por_id(eleccion_id) {
                Some(eleccion) => {
                    if block_timestamp > eleccion.fecha_final {
                        return Err(String::from("La votación ya finalizó."));
                    }
                    if eleccion.votacion_iniciada {
                        return Err(String::from("La votación ya inició."));
                    }
                    if block_timestamp < eleccion.fecha_inicio {
                        return Err(String::from("Todavía no es la fecha para la votación."));
                    }
                    eleccion.votacion_iniciada = true;
                    return Ok(String::from("Se inició la votación exitosamente."));
                },
                None => return Err(String::from("No existe una elección con ese id."))
            }
        }

        /// Permite al administrador obtener información del siguiente usuario pendiente en una elección específica.
        /// Obtiene información del siguiente usuario pendiente en una elección.
        /// - eleccion_id: u64 - ID de la elección de la cual se desea obtener el siguiente usuario pendiente
        /// Result<String, String>: Mensaje con los datos del usuario pendiente o un mensaje de error si no hay usuarios pendientes.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Obtiene la elección correspondiente al ID proporcionado y luego obtiene la información del primer usuario pendiente
        /// en la lista de usuarios pendientes de esa elección. Retorna un mensaje con los datos del usuario pendiente si existe,
        /// indicando si desea ser un VOTANTE o un CANDIDATO. Retorna un mensaje de error si no hay usuarios pendientes.
        #[ink(message)]
        pub fn obtener_siguiente_usuario_pendiente_en_una_eleccion(&mut self, eleccion_id:u64) -> Result<String, String>
        {
            self.obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(eleccion_id)
        }
        pub fn obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(&mut self, eleccion_id:u64) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let eleccion_elegida = match self.obtener_eleccion_por_id(eleccion_id) {
                Some(eleccion) => eleccion,
                None => return Err(String::from("Eleccion no encontrada")),
            };
            let sig_usuario = eleccion_elegida.usuarios_pendientes.first();
            match sig_usuario {
                Some(usuario_eleccion) => {
                    let mut datos_usuario = String::from("Usuario: ");
                    datos_usuario.push_str( hex::encode(usuario_eleccion.0).as_str() );
                    match usuario_eleccion.1 {
                        TIPO_DE_USUARIO::VOTANTE => datos_usuario.push_str("\nEl usuario quiere ser un VOTANTE"),
                        TIPO_DE_USUARIO::CANDIDATO => datos_usuario.push_str("\nEl usuario quiere ser un CANDIDATO")
                    };
                    Ok(datos_usuario)
                },
                None => Err(String::from("No hay usuarios pendientes.")),
            }
        }
        
        /// Permite al administrador procesar el siguiente usuario pendiente en una elección específica.
        /// Parámetros:
        /// - eleccion_id: u64 - ID de la elección en la que se procesarán los usuarios pendientes.
        /// - aceptar_usuario: bool - Indica si se acepta (true) o se rechaza (false) al usuario pendiente.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Obtiene la elección correspondiente al ID proporcionado. Luego, procesa el siguiente usuario pendiente
        /// en la elección según la acción indicada (aceptar o rechazar) y retorna el resultado del procesamiento.
        #[ink(message)]
        pub fn procesar_usuarios_en_una_eleccion(&mut self, eleccion_id:u64,aceptar_usuario:bool) -> Result<String, String>
        {
            self.procesar_usuarios_en_una_eleccion_privado(eleccion_id,aceptar_usuario)
        }
        pub fn procesar_usuarios_en_una_eleccion_privado(&mut self, eleccion_id:u64,aceptar_usuario:bool) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            
            let eleccion_elegida = match self.obtener_eleccion_por_id(eleccion_id) {
                Some(eleccion) => eleccion,
                None => return Err(String::from("Eleccion no encontrada")),
            };
            return eleccion_elegida.procesar_siguiente_usuario_pendiente(aceptar_usuario);
        }

        // ====-----==== METODOS PÚBLICOS ====----====

        /// Ingresa al usuario actual a una elección como pendiente de aprobación.
        /// - eleccion_id: u64 - ID de la elección a la que se desea ingresar.
        /// - tipo: TIPO_DE_USUARIO - Tipo de usuario que desea ingresar (VOTANTE o CANDIDATO).
        /// Descripción:
        /// Verifica si el usuario está registrado. Obtiene el ID del usuario actual y el timestamp actual del bloque.
        /// Valida el estado actual de la elección y verifica si el usuario ya ha sido rechazado previamente o si ya está pendiente.
        /// Luego, agrega al usuario como pendiente en la elección y retorna un mensaje de éxito.
        #[ink(message)]
        pub fn ingresar_a_eleccion(&mut self, eleccion_id:u64, tipo:TIPO_DE_USUARIO) -> Result<String, String>
        {
            self.ingresar_a_eleccion_privado(eleccion_id,tipo)
        }
        fn ingresar_a_eleccion_privado(&mut self, eleccion_id:u64, tipo:TIPO_DE_USUARIO) -> Result<String, String>
        {
            if !self.es_usuario_registrado() { return Err(ERRORES::USUARIO_NO_REGISTRADO.to_string()); }
            let id = self.env().caller();

            let block_timestamp = self.env().block_timestamp();
            let result = self.validar_estado_eleccion(eleccion_id, block_timestamp, id);
            let eleccion = match result {
                Ok(eleccion) => eleccion,
                Err(mensaje) => return Err(mensaje)
            };
            //Validar que un usuario que ya ha sido rechazado en la misma eleccion no intente volver a ponerse como pendiente 
            if eleccion.usuarios_rechazados.contains(&id) {
                return Err(String::from("Ya has sido rechazado no puedes ingresar a la eleccion"));
            }
            
            if eleccion.contiene_usuario_pendiente(id){
                return Err(String::from("No puedes ingresar dos veces a la misma eleccion"));
            }

            eleccion.usuarios_pendientes.push((id,tipo));

            return Ok(String::from("Ingresó a la elección correctamente Pendiente de aprobacion del Administrador"));
        }
        
        /// Realiza el voto a un candidato en una elección específica.
        /// - eleccion_id: u64 - ID de la elección.
        /// - candidato_id: u32 - ID del candidato.
        /// Descripción:
        /// Verifica si el usuario está registrado. Obtiene el ID del votante y el timestamp actual del bloque.
        /// Luego, verifica si la elección existe y si la votación está activa y en el período correcto.
        /// Finalmente, llama al método de votar_candidato de la elección y retorna su resultado.
        #[ink(message)]
        pub fn votar_a_candidato(&mut self, eleccion_id:u64, candidato_id:u32) -> Result<String, String>
        {
            self.votar_a_candidato_privado(eleccion_id, candidato_id)
        }
        fn votar_a_candidato_privado(&mut self, eleccion_id:u64, candidato_id:u32) -> Result<String, String>
        {
            if !self.es_usuario_registrado() { return Err(ERRORES::USUARIO_NO_REGISTRADO.to_string()); }
            let id = self.env().caller();
            let block_timestamp = self.env().block_timestamp();
    
            match self.obtener_eleccion_por_id(eleccion_id) {
                Some(eleccion) => {
                    if !eleccion.votacion_iniciada {
                        if block_timestamp < eleccion.fecha_inicio {
                            return Err(String::from("Todavía no es la fecha para la votación."));
                        }
                        eleccion.votacion_iniciada = true;
                    }
                    if block_timestamp > eleccion.fecha_final {
                        return Err(String::from("La votación ya finalizó."));
                    }
    
                    return eleccion.votar_candidato(id, candidato_id);
                },
                None => return Err(String::from("No existe una elección con ese id."))
            }
        }

        /// Obtiene la información de un candidato en una elección específica.
        /// - eleccion_id: u64 - ID de la elección.
        /// - candidato_id: u32 - ID del candidato.
        /// Result<String, String>: Información del candidato o mensaje de error.
        /// Descripción:
        /// Busca la elección por su ID y luego obtiene la información del candidato.
        /// Retorna un mensaje con el nombre, apellido y DNI del candidato si está registrado;
        /// de lo contrario, retorna un error.
        #[ink(message)]
        pub fn obtener_informacion_candidato_eleccion(&self, eleccion_id:u64, candidato_id:u32) -> Result<String, String>
        {
            self.obtener_informacion_candidato_eleccion_privado(eleccion_id, candidato_id)
        }
        fn obtener_informacion_candidato_eleccion_privado(&self, eleccion_id:u64, candidato_id:u32) -> Result<String, String>
        {
            let eleccion_elegida = match self.obtener_ref_eleccion_por_id(eleccion_id) {
                Some(eleccion) => eleccion,
                None => return Err(String::from("Eleccion no encontrada.")),
            };
            let candidato_info = match eleccion_elegida.obtener_informacion_candidato(candidato_id) {
                Some(candidato) => candidato,
                None => return Err(String::from("Candidato no encontrado.")),
            };

            let info = self.obtener_usuario(candidato_info.id).expect("No se pudo encontrar la información del candidato."); // Si se encuentra el candidato, DEBE estar registrado
            let mut str = String::from("Nombre: ") + info.nombre.as_str();
            str.push_str((String::from("\nApellido: ") + info.apellido.as_str()).as_str());
            str.push_str((String::from("\nDNI: ") + info.dni.as_str()).as_str());
            return Ok(str);
        }

        // ====-----==== METODOS USADOS POR EL ADMINISTRADOR ====----====
        // ====-----==== METODOS USADOS POR EL ADMINISTRADOR ====----====
        // ====-----==== METODOS USADOS POR EL ADMINISTRADOR ====----====

        /// Utilizado por un administrador.
        /// Activa el registro de usuarios.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Luego, verifica si el registro ya está activado. Si es así, retorna un error. Si no, activa el registro 
        /// y retorna un mensaje de éxito.
        #[ink(message)]
        pub fn activar_registro(&mut self) -> Result<String, String>
        {
            self.activar_registro_privado()
        }
        fn activar_registro_privado(&mut self) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            if self.registro_activado { return Err(String::from("El registro ya está activado.")); }
            self.registro_activado = true;
            return Ok(String::from("Se activó el registro para los usuarios."));
        }

        /// Utilizado por un administrador.
        /// Desactiva el registro de usuarios si no está activo el registro.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        /// Luego, verifica si el registro ya está desactivado. Si es así, retorna un error. Si no, desactiva el
        /// registro y retorna un mensaje de éxito.
        #[ink(message)]
        pub fn desactivar_registro(&mut self) -> Result<String, String>
        {
            self.desactivar_registro_privado()
        }
        fn desactivar_registro_privado(&mut self) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            if !self.registro_activado { return Err(String::from("El registro ya está desactivado.")); }
            self.registro_activado = false;
            return Ok(String::from("Se desactivó el registro para los usuarios."));
        }

        /// Utilizado por el administrador.
        /// Permite al administrador transferir el rol de administrador a otro usuario.
        /// id: AccountId: ID del nuevo administrador.
        /// Descripción:
        ///Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        ///  Si es administrador, asigna el ID proporcionado como el nuevo administrador y retorna un mensaje de éxito.
        #[ink(message)]
        pub fn transferir_administrador(&mut self, id:AccountId) -> Result<String, String>
        {
            self.transferir_administrador_privado(id)
        }
        pub fn transferir_administrador_privado(&mut self, id:AccountId) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            self.administrador = id;
            return Ok(String::from("Se transfirió el rol de administrador correctamente."));
        }
        
        /// Utilizado por el administrador.
        /// Permite al administrador asignar un generador de reportes.
        /// id: AccountId: ID del nuevo generador de reportes.
        /// Descripción:
        /// Verifica si el llamador es un administrador. Si no, retorna un error indicando que no es administrador.
        ///  Si es administrador, asigna el ID proporcionado como el nuevo generador de reportes y retorna un mensaje de éxito.
        #[ink(message)]
        pub fn asignar_generador_reportes(&mut self, id:AccountId) -> Result<String, String>
        {
            self.asignar_generador_reportes_privado(id)
        }
        pub fn asignar_generador_reportes_privado(&mut self, id:AccountId) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            self.generador_reportes = Some(id);
            return Ok(String::from("Se asigno el generador reportes correctamente."));
        }

        // ====-----==== METODOS PARA EL GENERADOR DE REPORTES ====----====
        // ====-----==== METODOS PARA EL GENERADOR DE REPORTES ====----====
        // ====-----==== METODOS PARA EL GENERADOR DE REPORTES ====----====


        /// Utilizado por el generador de reportes asignado por el administrador.
        /// Obtiene la informacion del usuario especificado por AccountId
        /// user_id: AccountId: ID del usuario.
        /// Option<(String, String, String)>: Tupla con nombre, apellido y DNI del usuario, o None si no se encuentra o el llamador no es el generador de reportes.
        /// Verifica si el llamador tiene permiso para generar reportes. Si no, retorna None. Luego, busca el usuario por su ID en la lista de usuarios.
        /// Si el usuario es encontrado, retorna su nombre, apellido y DNI como una tupla. Si no, retorna None.
        #[ink(message)]
        pub fn obtener_informacion_usuario(&self, user_id: AccountId) -> Option<(String, String, String)> 
        {
            self.obtener_informacion_usuario_privado(user_id)
        }
        pub fn obtener_informacion_usuario_privado(&self, user_id: AccountId) -> Option<(String, String, String)> 
        {
            if !self.es_generador_reportes() && !self.es_administrador() {return None;}

            let option_usuario = self.usuarios.iter().find(|usuario| usuario.id == user_id);
            match option_usuario {
                None => None,
                Some(usuario) => Some( (usuario.nombre.clone(),  usuario.apellido.clone(),  usuario.dni.clone()) )
            }

        }

        /// Utilizado por el generador de reportes asignado por el administrador.
        /// Obtiene la lista de votantes y su estado de voto para una elección específica.
        /// eleccion_id: u64: ID de la elección.
        /// Result<Vec<(AccountId,u32)>, String>: Vector con el ID de cada candidato y su total de votos, o un mensaje de error.
        /// Descripción:
        /// La función verifica si el usuario es el generador de reportes. Si no lo es, devuelve un error. Obtiene la marca de tiempo
        ///  del bloque actual y verifica si la elección ha finalizado. Si la elección no ha terminado, devuelve un error. Si la elección
        ///  existe y ha finalizado, devuelve la lista de votantes con su estado de voto. Si la elección no existe, devuelve un error.
        #[ink(message)]
        pub fn obtener_votantes_eleccion_por_id(&mut self, eleccion_id: u64) -> Result<Vec<(AccountId,bool)>, String>
        {
            self.obtener_votantes_eleccion_por_id_privado(eleccion_id)
        }
        pub fn obtener_votantes_eleccion_por_id_privado(&mut self, eleccion_id: u64) -> Result<Vec<(AccountId,bool)>, String>
        {
            if !self.es_generador_reportes() && !self.es_administrador() { return Err(String::from("No es el generador de reportes o no es el administrador!")); }
            let block_timestamp = self.env().block_timestamp();
            
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    if eleccion.fecha_final > block_timestamp {
                        return Err(String::from("La elección no finalizó, no puedes obtener los datos."));
                    }
                    
                    Ok(eleccion.votantes.iter().map(|votante| (votante.id, votante.voto_emitido)).collect())
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }


        /// Utilizado por el generador de reportes asignado por el administrador.
        /// Obtiene la lista de candidatos y sus votos para una elección específica.
        /// eleccion_id: u64: ID de la elección.
        /// Result<Vec<(AccountId,u32)>, String>: Vector con el ID de cada candidato y su total de votos, o un mensaje de error.
        /// Descripción:
        /// La función verifica si el usuario es el generador de reportes. Si no lo es, devuelve un error. Obtiene la marca de tiempo del
        /// bloque actual y verifica si la elección ha finalizado. Si la elección no ha terminado, devuelve un error. Si la elección existe
        /// y ha finalizado, devuelve la lista de candidatos con sus votos. Si la elección no existe, devuelve un error.
        #[ink(message)]
        pub fn obtener_candidatos_eleccion_por_id(&mut self, eleccion_id: u64) -> Result<Vec<(AccountId,u32)>, String>
        {
            self.obtener_candidatos_eleccion_por_id_privado(eleccion_id)
        }
        pub fn obtener_candidatos_eleccion_por_id_privado(&mut self, eleccion_id: u64) -> Result<Vec<(AccountId,u32)>, String>
        {
            if !self.es_generador_reportes() && !self.es_administrador() { return Err(String::from("No es el generador de reportes o no es el administrador!")); }
            let block_timestamp = self.env().block_timestamp();

            match self.obtener_eleccion_por_id(eleccion_id){
                Some(eleccion) => {
                    if eleccion.fecha_final > block_timestamp {
                        return Err(String::from("La elección no finalizó, no puedes obtener los datos."));
                    }
                    
                    Ok(eleccion.candidatos.iter().map(|candidato| (candidato.id, candidato.votos_totales)).collect())
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }

        /// Obtiene los resultados de una elección específica.
        /// eleccion_id: u64: ID de la elección.
        /// Result<Vec<(AccountId,u32)>, String>: Vector con el ID de cada candidato y su total de votos, o un mensaje de error.
        /// Descripción:
        /// La función obtiene la marca de tiempo del bloque actual y busca la elección por su ID. Si no encuentra la elección,
        /// devuelve un error. Luego, intenta obtener los resultados de la votación usando la marca de tiempo. Si los resultados
        /// aún no están disponibles, devuelve un error. Si están disponibles, devuelve una copia de los resultados.
        #[ink(message)]
        pub fn obtener_resultados(&mut self, eleccion_id:u64) -> Result<Resultados, String> 
        {
            self.obtener_resultados_privado(eleccion_id)
        }
        fn obtener_resultados_privado(&mut self,eleccion_id: u64) -> Result<Resultados, String> 
        {
            let block_timestamp= self.env().block_timestamp();
            let eleccion = match self.obtener_eleccion_por_id(eleccion_id){
                Some(eleccion) => eleccion,
                None => return Err(String::from("No se encontró una elección con ese id."))
            };
    
            match eleccion.obtener_resultados_votacion(block_timestamp) {
                None => Err(String::from("Todavía no están los resultados de la elección publicados.")),
                Some(resultados) => Ok(resultados.clone())
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::codegen::Env;
        use ink::env::test::advance_block;
        use ink::env::test::{ default_accounts, set_block_timestamp, DefaultAccounts };
        use ink::env::DefaultEnvironment;
        
        fn get_default_test_accounts() -> DefaultAccounts<ink::env::DefaultEnvironment> 
        {
            default_accounts::<ink::env::DefaultEnvironment>()
        }
    
        fn set_caller(caller: AccountId) 
        {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
        }

        fn set_contract(caller: AccountId) 
        {
            ink::env::test::set_contract::<ink::env::DefaultEnvironment>(caller);
        }

        fn setup_eleccion() -> Eleccion 
        {
            Eleccion {
                id: 1,
                candidatos: Vec::new(),
                votantes: Vec::new(),
                usuarios_rechazados: Vec::new(),
                usuarios_pendientes: Vec::new(),
                votacion_iniciada: false,
                fecha_inicio: 0,
                fecha_final: 0,
                resultados:None
            }
        }

        fn crear_usuario(id: AccountId, nombre: &str, apellido: &str, dni: &str) -> Usuario 
        {
            Usuario {
                id,
                nombre: nombre.to_string(),
                apellido: apellido.to_string(),
                dni: dni.to_string(),
            }
        }

        fn crear_trabajo_final(administrador: AccountId) -> SistemaElecciones 
        {
            SistemaElecciones {
                administrador,
                generador_reportes: None,
                registro_activado: false,
                usuarios: Vec::new(),
                usuarios_pendientes: Vec::new(),
                usuarios_rechazados: Vec::new(),
                elecciones: Vec::new(),
            }
        }
    
        // ====================== INICIO TESTS ERRORES ======================
        // ====================== INICIO TESTS ERRORES ======================
        // ====================== INICIO TESTS ERRORES ======================

        #[test]
        fn test_no_es_administrador() 
        {
            let error = ERRORES::NO_ES_ADMINISTRADOR;
            assert_eq!(error.to_string(), "No eres el administrador.");
        }
    
        #[test]
        fn test_usuario_no_registrado() 
        {
            let error = ERRORES::USUARIO_NO_REGISTRADO;
            assert_eq!(error.to_string(), "No estás registrado en el sistema. Espera a que te acepten en el mismo o realiza la solicitud.");
        }
            
        #[test]
        fn test_error_usuario_no_registrado() 
        {
            let error = ERRORES::USUARIO_NO_REGISTRADO;
            assert_eq!(error.to_string(), "No estás registrado en el sistema. Espera a que te acepten en el mismo o realiza la solicitud.");
        }
    
        // ====================== INICIO TESTS ELECCION ======================
        // ====================== INICIO TESTS ELECCION ======================
        // ====================== INICIO TESTS ELECCION ======================

        #[test]
        fn test_obtener_resultados_votacion() 
        {
            let administrador: AccountId = AccountId::from([0x1; 32]);
            set_caller(administrador);

            let mut contrato = SistemaElecciones::new();
            set_block_timestamp::<ink::env::DefaultEnvironment>(1675666400000);

            set_contract(administrador);

            let _ = contrato.activar_registro();
            let _ = contrato.crear_eleccion_privado(
                "01-01-2023 12:00".to_string(),
                "31-01-2025 12:00".to_string()
            );
            let _ = contrato.crear_eleccion_privado(
                "01-01-2023 12:00".to_string(),
                "31-01-2023 12:00".to_string()
            );

            let eleccion = contrato.obtener_eleccion_por_id(1).unwrap();
            assert!(eleccion.obtener_resultados_votacion(1675666400000).is_none()); // No obtiene resultados porque la elección no finalizó

            let eleccion = contrato.obtener_eleccion_por_id(2).unwrap();
            let resultados = Resultados {votos_totales:0, votos_candidatos:Vec::new(), votos_realizados: 0 };
            assert_eq!(eleccion.obtener_resultados_votacion(1675666400000), Some(&resultados)); // Obtiene resultados, procesandolos por ser la primera vez
            assert_eq!(eleccion.obtener_resultados_votacion(1675666400000), Some(&resultados)); // Obtiene de nuevo los resultados, pero no los procesa porque ya los tiene
        }

        #[ink::test]
        fn test_desactivar_registro() 
        {
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]); // Ejemplo de ID de administrador

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Establece el llamante como administrador (reemplaza con el AccountId real)
            set_caller(id_administrador);

            // Llama a la función del contrato que requiere env().caller()
            let result = contrato.desactivar_registro();

            // Asegúrate de que el resultado sea el esperado
            assert_eq!(result, Err("El registro ya está desactivado.".to_string()));
        }

        #[ink::test]
        fn test_obtener_resultados_privado_exito() 
        {
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]); 
            let id_candidato_1 = AccountId::from([0x06; 32]);
            let id_candidato_2 = AccountId::from([0x06; 32]);
            let id_candidato_3 = AccountId::from([0x06; 32]);
            let id_candidato_4 = AccountId::from([0x06; 32]);

            let vector_votos_candidatos = vec![
                (id_candidato_1, 13),
                (id_candidato_4, 7),
                (id_candidato_2, 3),
                (id_candidato_3, 2)
            ];

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Simular una elección válida con usuarios pendientes
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 50, 
                fecha_final: 100, 
                resultados: Some(Resultados { 
                    votos_totales: 30, 
                    votos_realizados: 25,
                    votos_candidatos: vector_votos_candidatos,
                }),
            });

            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(705);

            let result_obtener = contrato.obtener_resultados_privado(1);
            
            // Verificar el resultado esperado
            assert!(result_obtener.is_ok(), "Error al obtener los resultados de la elección");
        }

        #[ink::test]
        fn test_obtener_resultados_privado_fallo_eleccion_inexistente() 
        {
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]);

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Simular una elección 
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 0, 
                fecha_final: 100, 
                resultados: None,
            });

            let result_obtener = contrato.obtener_resultados_privado(10);

            // Verificar el resultado esperado
            assert!(result_obtener.is_err(), "Se esperaba un error al obtener los resultados de la elección (no se encontró elección)");
        }

        #[ink::test]
        fn test_obtener_resultados_privado_fallo_no_finalizo() 
        {
            // Configuración inicial del contrato y del contexto de prueba
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]); // Ejemplo de ID de administrador

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Simular una elección válida sin usuarios pendientes
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 50, // Ajustar según la lógica de tu contrato
                fecha_final: 100, // Ajustar según la lógica de tu contrato
                resultados: None,
            });
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(75);

            // Ejecutar el método para obtener el siguiente usuario pendiente en la elección
            let result_obtener = contrato.obtener_resultados_privado(1);

            // Verificar el resultado esperado
            assert!(result_obtener.is_err(), "Se esperaba un error al obtener el siguiente usuario pendiente en la elección (la eleccion no finalizo)");
        }

        #[ink::test]
        fn test_obtener_informacion_usuario_privado()
        {
            let generador_reportes = AccountId::from([0; 32]);
            let administrador = AccountId::from([1; 32]);
            let user_id = AccountId::from([2; 32]);
            let random_user = AccountId::from([12; 32]);
            set_caller(administrador);
            let mut contrato = SistemaElecciones::new();
            assert!(contrato.asignar_generador_reportes(generador_reportes).is_ok());

            set_caller(generador_reportes);
            //Usuario no existente
            let resultado = contrato.obtener_informacion_usuario_privado(user_id);
            assert!(resultado.is_none());
            
            let user = Usuario { id: user_id, nombre: "Joaquin".to_string(), apellido: "Fontana".to_string(), dni: "22222".to_string() };
            let nombre = user.nombre.clone();
            let apellido = user.apellido.clone();
            let dni = user.dni.clone();
            contrato.usuarios.push(user);
            
            //Intentar llamar sin ser el generador reporte o admin
            set_caller(random_user);
            assert!(contrato.obtener_informacion_usuario_privado(user_id).is_none());
            
            set_caller(administrador);
            assert!(contrato.obtener_informacion_usuario_privado(user_id).is_some_and(|tupla| tupla.0 == nombre && tupla.1 == apellido && tupla.2 == dni));
            
            set_caller(generador_reportes);
            assert!(contrato.obtener_informacion_usuario_privado(user_id).is_some_and(|tupla| tupla.0 == nombre && tupla.1 == apellido && tupla.2 == dni));
        }

        #[test]
        fn test_obtener_votantes_eleccion_por_id_privado() 
        {
            let administrador: AccountId = AccountId::from([0x1; 32]);
            set_caller(administrador);

            let mut contrato = SistemaElecciones::new();
            set_block_timestamp::<ink::env::DefaultEnvironment>(1675666400000);

            set_contract(administrador);
            
            let _ = contrato.activar_registro();
            let _ = contrato.crear_eleccion_privado(
                "01-01-2023 12:00".to_string(),
                "31-01-2025 12:00".to_string()
            );
            let _ = contrato.crear_eleccion_privado(
                "01-01-2023 12:00".to_string(),
                "31-01-2023 12:00".to_string()
            );

            assert!(contrato.obtener_votantes_eleccion_por_id_privado(1).is_err()); // No se obtienen resultados porque no es el generador de reportes
            let _ = contrato.asignar_generador_reportes_privado(administrador);
            assert!(contrato.obtener_votantes_eleccion_por_id_privado(1).is_err()); // No se obtienen resultados porque la elección no finalizó
            
            let err = contrato.obtener_votantes_eleccion_por_id_privado(2); // Se obtienen resultados porque es el generador de reportes y finalizó la elección
            assert!(err.is_ok());

            let err = contrato.obtener_votantes_eleccion_por_id_privado(3); // No se obtienen resultados porque no existe la elección
            assert!(err.is_err());
        }

        #[test]
        fn test_contiene_usuario_pendiente() 
        {
            let mut eleccion = setup_eleccion();
            let accounts: DefaultAccounts<DefaultEnvironment> = get_default_test_accounts(); // Cuentas predeterminadas utilizadas para tests
            
            eleccion.usuarios_pendientes.push((accounts.alice, TIPO_DE_USUARIO::VOTANTE));
            
            assert!(eleccion.contiene_usuario_pendiente(accounts.alice));
            assert!(!eleccion.contiene_usuario_pendiente(accounts.bob));
        }
    
        #[test]
        fn test_existe_candidato() 
        {
            let mut eleccion = setup_eleccion();
            let accounts = get_default_test_accounts();
            
            eleccion.candidatos.push(CandidatoConteo {
                id: accounts.alice,
                candidato_id: 1,
                votos_totales: 0,
            });
    
            assert!(eleccion.existe_candidato(1));
            assert!(!eleccion.existe_candidato(2));
        }
    
        #[test]
        fn test_votar_candidato() 
        {
            let mut eleccion = setup_eleccion();
            let accounts = get_default_test_accounts();
    
            eleccion.candidatos.push(CandidatoConteo {
                id: accounts.alice,
                candidato_id: 1,
                votos_totales: 0,
            });
    
            eleccion.votantes.push(Votante {
                id: accounts.bob,
                voto_emitido: false,
            });
    
            let result = eleccion.votar_candidato(accounts.bob, 1);
            assert_eq!(result, Ok(String::from("Voto emitido exitosamente.")));
            assert!(eleccion.votantes[0].voto_emitido);
            assert_eq!(eleccion.candidatos[0].votos_totales, 1);
        }
    
        #[test]
        fn test_procesar_siguiente_usuario_pendiente() 
        {
            let mut eleccion = setup_eleccion();
            let accounts = get_default_test_accounts();
    
            eleccion.usuarios_pendientes.push((accounts.alice, TIPO_DE_USUARIO::VOTANTE));
            eleccion.usuarios_pendientes.push((accounts.bob, TIPO_DE_USUARIO::CANDIDATO));
    
            let result = eleccion.procesar_siguiente_usuario_pendiente(true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
            assert_eq!(eleccion.votantes.len(), 1);
            assert_eq!(eleccion.candidatos.len(), 0);
    
            let result = eleccion.procesar_siguiente_usuario_pendiente(true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
            assert_eq!(eleccion.votantes.len(), 1);
            assert_eq!(eleccion.candidatos.len(), 1);
    
            let result = eleccion.procesar_siguiente_usuario_pendiente(false);
            assert_eq!(result, Err(String::from("No hay usuarios pendientes.")));
        }
    
        #[test]
        fn test_obtener_informacion_candidato() 
        {
            let administrador: AccountId = AccountId::from([0x1; 32]);
            let bob: AccountId = AccountId::from([0x2; 32]);
            set_caller(administrador);

            let mut contrato = SistemaElecciones::new();

            let _ = contrato.activar_registro();

            let _ = contrato.crear_eleccion(
                String::from("01-07-2024 12:00"),
                String::from("31-12-2024 12:00"),
            );

            set_caller(bob);

            let _ = contrato.registrarse(String::from("Bob"), String::from("Bob"), String::from("Bob"));

            set_caller(administrador);

            let _ = contrato.procesar_siguiente_usuario_pendiente(true);

            set_caller(bob);

            let _ = contrato.ingresar_a_eleccion_privado(1, TIPO_DE_USUARIO::CANDIDATO);

            set_caller(administrador);

            let _ = contrato.procesar_usuarios_en_una_eleccion_privado(1, true);

            set_caller(bob);

            // El código de arriba simúla la activación del registro, creación de una elección, registro de una persona, 
            // aceptación del registro de una persona, registro de una persona a una elección y aceptación de ese registro en la elección

            let res = contrato.obtener_informacion_candidato_eleccion(1, 1);
            assert!(res.is_ok()); // Se obtiene la información del candidato porque existe la elección y existe el candidato

            let res = contrato.obtener_informacion_candidato_eleccion(2, 1);
            assert!(res.is_err()); // No se obtiene la información porque la elección no existe

            let res = contrato.obtener_informacion_candidato_eleccion(1, 2);
            assert!(res.is_err()); // No se obtiene la información porque el candidato no existe
        }

        #[test]
        fn test_procesar_siguiente_usuario_pendiente_eleccion() 
        {
            let mut eleccion = setup_eleccion();
            let accounts = default_accounts::<DefaultEnvironment>();
    
            eleccion.usuarios_pendientes.push((accounts.alice, TIPO_DE_USUARIO::VOTANTE));
            eleccion.usuarios_pendientes.push((accounts.bob, TIPO_DE_USUARIO::CANDIDATO));
    
            let result = eleccion.procesar_siguiente_usuario_pendiente(true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
            assert_eq!(eleccion.votantes.len(), 1);
            assert_eq!(eleccion.candidatos.len(), 0);
    
            let result = eleccion.procesar_siguiente_usuario_pendiente(true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
            assert_eq!(eleccion.votantes.len(), 1);
            assert_eq!(eleccion.candidatos.len(), 1);
    
            let result = eleccion.procesar_siguiente_usuario_pendiente(false);
            assert_eq!(result, Err(String::from("No hay usuarios pendientes.")));
        }

        #[test]
        fn test_votar_a_candidato_privado() 
        {
            let accounts = get_default_test_accounts();
            let alice = accounts.alice;
            let charlie = accounts.charlie;
            let bob = accounts.bob;

            set_caller(alice);
            let mut contrato = SistemaElecciones::new();
            let nueva_eleccion = contrato.crear_eleccion_privado(
                String::from("01-07-2024 12:00"),
                String::from("31-12-2024 12:00"),
            );
            assert_eq!(nueva_eleccion, Ok(format!("Eleccion creada exitosamente. Id de la elección: 1")) );
            contrato.activar_registro_privado();

            // Usuarios se registran
            set_caller(bob);
            let result = contrato.registrarse_privado("Bob".to_string(), "Asd".to_string(), "12345678".to_string() );
            assert_eq!(result, Ok("Registro exitoso. Se te añadió en la cola de usuarios pendientes.".to_string()) );
            set_caller(charlie);
            contrato.registrarse_privado("Charlie".to_string(), "Dsa".to_string(), "87654321".to_string() );
            assert_eq!(result, Ok("Registro exitoso. Se te añadió en la cola de usuarios pendientes.".to_string()) );

            // Alice aprueba registro en sistema
            set_caller(alice);
            contrato.procesar_siguiente_usuario_pendiente_privado(true);
            contrato.procesar_siguiente_usuario_pendiente_privado(true);
        
            // Usuarios se registran en eleccion
            set_caller(bob);
            contrato.ingresar_a_eleccion_privado(1, TIPO_DE_USUARIO::CANDIDATO);
            set_caller(charlie);
            contrato.ingresar_a_eleccion_privado(1, TIPO_DE_USUARIO::VOTANTE);

            set_caller(alice);
            let result = contrato.procesar_usuarios_en_una_eleccion_privado(1, true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
            let result = contrato.procesar_usuarios_en_una_eleccion_privado(1, true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));

            // Vota en eleccion usuario no registrado
            set_caller(alice);
            let result = contrato.votar_a_candidato_privado(1, 1);
            assert_eq!(result, Err(String::from("No estás registrado en el sistema. Espera a que te acepten en el mismo o realiza la solicitud.")) );

            set_caller(charlie);
            // Vota en eleccion que todavia no inicio
            let result = contrato.votar_a_candidato_privado(1, 1);
            assert_eq!(result, Err(String::from("Todavía no es la fecha para la votación.")) );

            contrato.elecciones[0].votacion_iniciada = true;
            // Voto a candidato inexistente
            let result = contrato.votar_a_candidato_privado(1, 2);
            assert_eq!(result, Err(String::from("No existe un candidato con este id.")) );

            // Voto exitoso
            let result = contrato.votar_a_candidato_privado(1, 1);
            assert_eq!(result, Ok(String::from("Voto emitido exitosamente.")));

            // Volver a votar en una eleccion que ya voto
            let result = contrato.votar_a_candidato_privado(1, 1);
            assert_eq!(result, Err("No se realizó el voto porque ya votaste anteriormente.".to_string() ));

            // Vota eleccion inexistente
            let result = contrato.votar_a_candidato_privado(7, 1);
            assert_eq!(result, Err(String::from("No existe una elección con ese id.")) );
            
            // Usuario no registrado como votante trata de votar
            set_caller(bob);
            let result = contrato.votar_a_candidato_privado(1, 1);
            assert_eq!(result, Err(String::from("No estás registrado en la elección.")) );
        }

        #[test]
        fn test_obtener_informacion_siguiente_usuario_pendiente_privado() {
            let accounts = get_default_test_accounts();
            let alice = accounts.alice;
            let charlie = accounts.charlie;
            let bob = accounts.bob;

            set_caller(alice);
            let mut contrato = SistemaElecciones::new();
            contrato.activar_registro_privado();
            let result = contrato.obtener_informacion_siguiente_usuario_pendiente();
            assert_eq!(result, Err(String::from("No hay usuarios pendientes.")) );
            
            set_caller(bob);
            let result = contrato.registrarse_privado("Bob".to_string(), "Asd".to_string(), "12345678".to_string() );
            assert_eq!(contrato.usuarios_pendientes.len(), 1);
            
            set_caller(alice);
            let result = contrato.obtener_informacion_siguiente_usuario_pendiente();
            let string_bob = "Nombre: Bob\nApellido: Asd\nDNI: 12345678".to_string();
            assert_eq!(result, Ok(string_bob) );

            set_caller(charlie);
            let result = contrato.registrarse_privado("Charlie".to_string(), "Asd".to_string(), "12345678".to_string() );
            assert_eq!(contrato.usuarios_pendientes.len(), 2);

            set_caller(alice);
            // Aprobamos a Bob para ver la informacion de Charlie
            contrato.procesar_siguiente_usuario_pendiente_privado(true);

            let result = contrato.obtener_informacion_siguiente_usuario_pendiente();
            let string_charlie = "Nombre: Charlie\nApellido: Asd\nDNI: 12345678".to_string();
            assert_eq!(result, Ok(string_charlie) );

            // Aprobamos a todos los usuarios pendientes y volvemos a consultar
            contrato.procesar_siguiente_usuario_pendiente_privado(true);
            let result = contrato.obtener_informacion_siguiente_usuario_pendiente();
            assert_eq!(result, Err(String::from("No hay usuarios pendientes.")) );
        }

        // ====================== FIN TESTS ELECCION ======================
        // ====================== FIN TESTS ELECCION ======================
        // ====================== FIN TESTS ELECCION ======================
    
        // ====================== INICIO TESTS SISTEMA ELECCIONES ======================
        // ====================== INICIO TESTS SISTEMA ELECCIONES ======================
        // ====================== INICIO TESTS SISTEMA ELECCIONES ======================
        
        #[test]
        fn test_constructor() 
        {
            let accounts = get_default_test_accounts();
            let alice = accounts.alice;
            let charlie = accounts.charlie;
            set_caller(alice);
    
            let sistema_elecciones = SistemaElecciones::new();
            assert_eq!(sistema_elecciones.registro_activado, false);
            assert_eq!(sistema_elecciones.administrador, alice);
            assert_ne!(sistema_elecciones.administrador, charlie);
        }

        #[ink::test]
        fn test_obtener_informacion_candidato_eleccion() 
        {
            let mut eleccion = setup_eleccion();
            let accounts = default_accounts::<DefaultEnvironment>();
            eleccion.candidatos.push(CandidatoConteo {
                id: accounts.alice,
                candidato_id: 1,
                votos_totales: 0,
            });
            eleccion.candidatos.push(CandidatoConteo {
                id: accounts.bob,
                candidato_id: 2,
                votos_totales: 0,
            });

            let candidato_info = eleccion.obtener_informacion_candidato(1);
            assert!(candidato_info.is_some());
            assert_eq!(candidato_info.unwrap().id, accounts.alice);

            let candidato_info = eleccion.obtener_informacion_candidato(2);
            assert!(candidato_info.is_some());
            assert_eq!(candidato_info.unwrap().id, accounts.bob);
    
            let candidato_info = eleccion.obtener_informacion_candidato(3);
            assert!(candidato_info.is_none());
        }

        #[test]
        fn test_obtener_informacion_siguiente_usuario_pendiente() 
        {
            let administrador: AccountId = AccountId::from([0x1; 32]);
            let otro_usuario: AccountId = AccountId::from([0x2; 32]);
            set_caller(administrador);

            let mut contrato = SistemaElecciones::new();
            
            let usuario = Usuario { id: (otro_usuario), nombre: ("Joaquin".to_string()), apellido: ("Fontana".to_string()), dni: ("22222222".to_string()) };
            let mut str = String::from("Nombre: ") + usuario.nombre.as_str();
            str.push_str((String::from("\nApellido: ") + usuario.apellido.as_str()).as_str());
            str.push_str((String::from("\nDNI: ") + usuario.dni.as_str()).as_str());
            //Intentar obtener informacion sin usuarios pendientes
            let result = contrato.obtener_informacion_siguiente_usuario_pendiente();
            assert!(result.is_err());
            
            contrato.usuarios_pendientes.push(usuario);

            let result = contrato.obtener_informacion_siguiente_usuario_pendiente();
            assert!(result.is_ok_and(|info| info == str));
        }

        #[test]
        fn test_transferir_administrador() 
        {
            let accounts = get_default_test_accounts();
            let alice = accounts.alice;
            let charlie = accounts.charlie;
            let bob = accounts.bob;
            set_caller(alice);
    
            let mut sistema_elecciones = SistemaElecciones::new();
    
            let result = sistema_elecciones.transferir_administrador_privado(charlie);
            assert!(result.is_ok());
            assert_ne!(sistema_elecciones.administrador, alice);
            assert_eq!(sistema_elecciones.administrador, charlie);
    
            set_caller(bob);
            let result = sistema_elecciones.transferir_administrador_privado(charlie);
            assert!(result.is_err());
            assert_ne!(sistema_elecciones.administrador, alice);
            assert_eq!(sistema_elecciones.administrador, charlie);
        }

        #[ink::test]
        fn test_obtener_candidatos_eleccion_por_id()
        {
            let administrador: AccountId = AccountId::from([0x1; 32]);
            let generador_reportes: AccountId = AccountId::from([0x2; 32]);
            set_caller(administrador);
            let mut contrato = SistemaElecciones::new();
            assert!(contrato.asignar_generador_reportes(generador_reportes).is_ok());
            assert!(contrato.generador_reportes.is_some_and(|id| id == generador_reportes));
            let mut eleccion = setup_eleccion();
            let eleccion_id = eleccion.id;
            eleccion.fecha_final = contrato.env().block_timestamp();
            contrato.elecciones.push(eleccion);
            advance_block::<ink::env::DefaultEnvironment>();
            set_caller(generador_reportes);
            let resultado = contrato.obtener_candidatos_eleccion_por_id_privado(eleccion_id);

            // Verificar el resultado y manejar el error
            assert!(
                resultado.is_ok(),
                "Error al obtener candidatos para la elección: {:?}",
                resultado.unwrap_err().to_string()
            );
        }
    
        #[ink::test]
        fn test_obtener_usuario() 
        {
            let accounts = get_default_test_accounts();
            let id: AccountId = accounts.alice;
            let mut trabajo_final = SistemaElecciones::new();
            let usuario = crear_usuario(id, "Juan", "Perez", "12345678");
            trabajo_final.usuarios.push(usuario);
    
            let result = trabajo_final.obtener_usuario(id);
            assert!(result.is_some());
            let user = result.unwrap();
            assert_eq!(user.nombre, "Juan");
            assert_eq!(user.apellido, "Perez");
            assert_eq!(user.dni, "12345678");
        }
    
        #[ink::test]
        fn test_es_usuario_registrado() 
        {
            let accounts = get_default_test_accounts();
            let id: AccountId = accounts.alice;
            let mut trabajo_final = SistemaElecciones::new();
            let usuario = crear_usuario(id, "Juan", "Perez", "12345678");
            trabajo_final.usuarios.push(usuario);
    
            assert!(trabajo_final.es_usuario_registrado());
        }
    
        #[ink::test]
        fn test_es_usuario_pendiente() 
        {
            let accounts = get_default_test_accounts();
            let id: AccountId = accounts.alice;
            let mut trabajo_final = SistemaElecciones::new();
            let usuario = crear_usuario(id, "Juan", "Perez", "12345678");
            trabajo_final.usuarios_pendientes.push(usuario);
    
            assert!(trabajo_final.es_usuario_pendiente());
        }
    
        #[test]
        fn test_existe_eleccion() 
        {
            let id: AccountId = [0; 32].into();
            let mut trabajo_final = crear_trabajo_final(id);
            let eleccion = setup_eleccion();
            trabajo_final.elecciones.push(eleccion);
    
            assert!(trabajo_final.existe_eleccion(1));
            assert!(!trabajo_final.existe_eleccion(2));
        }
    
        #[test]
        fn test_obtener_eleccion_por_id() 
        {
            let id: AccountId = [0; 32].into();
            let mut trabajo_final = crear_trabajo_final(id);
            let eleccion = setup_eleccion();
            trabajo_final.elecciones.push(eleccion);
    
            let result = trabajo_final.obtener_eleccion_por_id(1);
            assert!(result.is_some());
            let eleccion_obtenida = result.unwrap();
            assert_eq!(eleccion_obtenida.id, 1);
    
            assert!(trabajo_final.obtener_eleccion_por_id(2).is_none());
        }


        #[ink::test]
        fn test_activar_registro() 
        {
            let mut contract = SistemaElecciones::new();

            let res = contract.activar_registro_privado();
            assert_eq!(res, Ok(String::from("Se activó el registro para los usuarios.")));
            assert_eq!(contract.registro_activado, true);

            let res = contract.activar_registro_privado();
            assert_eq!(res, Err(String::from("El registro ya está activado.")));
        }

        #[ink::test]
        fn test_registro_usuario() 
        {
            let administrador: AccountId = AccountId::from([0x1; 32]);
            let otro_usuario: AccountId = AccountId::from([0x2; 32]);

            set_caller(administrador);
            let mut contrato = SistemaElecciones::new();

            contrato.activar_registro_privado().unwrap();

            set_caller(otro_usuario);

            let resultado = contrato.registrarse_privado("John".to_string(), "Doe".to_string(), "12345678".to_string());
            assert_eq!(resultado, Ok("Registro exitoso. Se te añadió en la cola de usuarios pendientes.".to_string()));
        }

        #[ink::test]
        fn test_obtener_ref_eleccion_por_id() 
        {
            let mut contrato = SistemaElecciones::new();

            let candidato1 = CandidatoConteo {
                id: AccountId::from([0x01; 32]),
                candidato_id: 1,
                votos_totales: 0,
            };
            let candidato2 = CandidatoConteo {
                id: AccountId::from([0x02; 32]),
                candidato_id: 2,
                votos_totales: 0,
            };

            let votante1 = Votante {
                id: AccountId::from([0x03; 32]),
                voto_emitido: false,
            };
            let votante2 = Votante {
                id: AccountId::from([0x04; 32]),
                voto_emitido: false,
            };

            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![candidato1],
                votantes: vec![votante1],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 0,
                fecha_final: 0,
                resultados:None
            });

            contrato.elecciones.push(Eleccion {
                id: 2,
                candidatos: vec![candidato2],
                votantes: vec![votante2],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 0,
                fecha_final: 0,
                resultados:None
            });

            let resultado = contrato.obtener_ref_eleccion_por_id(1);
            assert!(resultado.is_some());
            assert_eq!(resultado.unwrap(), &contrato.elecciones[0]);

            let resultado = contrato.obtener_ref_eleccion_por_id(2);
            assert!(resultado.is_some());
            assert_eq!(resultado.unwrap(), &contrato.elecciones[1]);

            let resultado = contrato.obtener_ref_eleccion_por_id(3);
            assert!(resultado.is_none());
        }

        #[ink::test]
        fn test_validar_estado_eleccion() 
        {
            let mut contrato = SistemaElecciones::new();
            let usuario_id = AccountId::from([0x01; 32]);
            
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![(usuario_id, TIPO_DE_USUARIO::VOTANTE)],
                votacion_iniciada: false,
                fecha_inicio: 100,
                fecha_final: 200,
                resultados:None
            });

            contrato.elecciones.push(Eleccion {
                id: 2,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: true,
                fecha_inicio: 100,
                fecha_final: 200,
                resultados:None
            });

            contrato.elecciones.push(Eleccion {
                id: 3,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 100,
                fecha_final: 50,
                resultados:None
            });

            contrato.elecciones.push(Eleccion {
                id: 4,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 150,
                fecha_final: 200,
                resultados:None
            });

            // Caso 1: Usuario ya registrado
            let resultado = contrato.validar_estado_eleccion(1, 50, usuario_id);
            assert_eq!(resultado, Err(String::from("Ya está registrado en la elección.")));

            // Caso 2: Votación ya comenzó
            let resultado = contrato.validar_estado_eleccion(2, 50, usuario_id);
            assert_eq!(resultado, Err(String::from("La votación en la elección ya comenzó, no te puedes registrar.")));

            // Caso 3: Elección ya finalizó
            let resultado = contrato.validar_estado_eleccion(3, 52, usuario_id);
            assert_eq!(resultado, Err(String::from("La elección ya finalizó, no te puedes registrar.")));

            // Caso 4: Elección válida y no iniciada
            let resultado = contrato.validar_estado_eleccion(4, 50, usuario_id);
            assert!(resultado.is_ok());
            let eleccion = resultado.unwrap();
            assert_eq!(eleccion.id, 4);
        }

        #[ink::test]
        fn test_crear_eleccion() 
        {
            // Instanciar el contrato
            let mut contrato = SistemaElecciones::new();
    
            // Configurar el administrador
            let administrador = AccountId::from([0x1; 32]);
            contrato.administrador = administrador;
    
            // Crear una elección válida
            let resultado = contrato.crear_eleccion_privado(
                "01-01-2025 12:00".to_string(),
                "31-01-2025 12:00".to_string()
            );
    
            // Verificar que la elección se creó correctamente
            assert_eq!(resultado, Ok("Eleccion creada exitosamente. Id de la elección: 1".to_string()));
    
            // Verificar que la elección se añadió a la lista
            assert_eq!(contrato.elecciones.len(), 1);
            let eleccion = &contrato.elecciones[0];
            assert_eq!(eleccion.id, 1);
            assert_eq!(eleccion.fecha_inicio, 1735732800000); 
            assert_eq!(eleccion.fecha_final, 1738324800000);
    
            // Crear una elección con fecha inicial inválida
            let resultado = contrato.crear_eleccion_privado(
                "01-01-2025 12:00".to_string(),
                "invalid-date".to_string()
            );
            assert_eq!(resultado, Err("Error en el formato de la fecha final. Formato: dd-mm-YYYY hh:mm".to_string()));
    
            // Crear una elección con fecha final inválida
            let resultado = contrato.crear_eleccion_privado(
                "invalid-date".to_string(),
                "31-01-2025 12:00".to_string()
            );
            assert_eq!(resultado, Err("Error en el formato de la fecha inicial. Formato: dd-mm-YYYY hh:mm".to_string()));
    
            // Crear una elección sin ser administrador
            contrato.administrador = AccountId::from([0x2; 32]);
            let resultado = contrato.crear_eleccion_privado(
                "01-01-2025 12:00".to_string(),
                "31-01-2025 12:00".to_string()
            );
            assert_eq!(resultado, Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()));
        }

        #[ink::test]
        fn test_procesar_usuarios_en_una_eleccion() 
        {
            let mut contrato = SistemaElecciones::new();
            let accounts = default_accounts::<DefaultEnvironment>();
            
            contrato.crear_eleccion_privado(
                String::from("01-07-2024 12:00"),
                String::from("02-07-2024 12:00"),
            ).unwrap();

            
            contrato.elecciones[0].usuarios_pendientes.push((accounts.alice, TIPO_DE_USUARIO::VOTANTE));
            contrato.elecciones[0].usuarios_pendientes.push((accounts.bob, TIPO_DE_USUARIO::CANDIDATO));

    
            // Caso 1: Procesar siguiente usuario aceptando
            let result = contrato.procesar_usuarios_en_una_eleccion(1, true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
    
            // Caso 2: Procesar siguiente usuario aceptando como candidato
            let result = contrato.procesar_usuarios_en_una_eleccion(1, true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
    
            // Caso 3: Procesar siguiente usuario rechazando
            let result = contrato.procesar_usuarios_en_una_eleccion(1, false);
            assert_eq!(result, Err(String::from("No hay usuarios pendientes.")));
    
            // Caso 4: Intentar procesar usuario en una elección no existente
            let result = contrato.procesar_usuarios_en_una_eleccion(2, true);
            assert_eq!(result, Err(String::from("Eleccion no encontrada")));
        } 

        #[ink::test]
        fn test_ingresar_a_eleccion() 
        {
            let accounts = get_default_test_accounts();
            let alice = accounts.alice;
            let charlie = accounts.charlie;
            let eleccion_id: u64 = 1;
            let tipo_usuario: TIPO_DE_USUARIO = TIPO_DE_USUARIO::VOTANTE;
            set_caller(alice);
        
            let mut contract = SistemaElecciones::new();
        
            // Establecemos el administrador como el llamante y activamos el registro
            contract.activar_registro().unwrap();
            contract.crear_eleccion("01-01-2024 10:00".into(), "02-01-2024 10:00".into()).unwrap();
        
            // Usuario no registrado intenta ingresar a la elección
            set_caller(charlie);
            let result = contract.ingresar_a_eleccion_privado(eleccion_id, tipo_usuario.clone());
            assert_eq!(result, Err(ERRORES::USUARIO_NO_REGISTRADO.to_string()), "Error: Usuario no registrado");
        
            // Registramos al usuario
            let result = contract.registrarse("Juan".into(), "Perez".into(), "12345678".into()).unwrap();
            assert_eq!(result, String::from("Registro exitoso. Se te añadió en la cola de usuarios pendientes."));
        
            // Aceptamos al usuario pendiente
            set_caller(alice);
            let result = contract.procesar_siguiente_usuario_pendiente(true);
            assert_eq!(result, Ok(String::from("Usuario agregado exitosamente.")));
        
            // Usuario registrado intenta ingresar a la elección
            set_caller(charlie);
            let result = contract.ingresar_a_eleccion_privado(eleccion_id, tipo_usuario.clone());
            assert_eq!(result, Ok(String::from("Ingresó a la elección correctamente Pendiente de aprobacion del Administrador")));
        
            
            // Limpiamos el estado de usuarios rechazados para continuar con el test
            contract.elecciones[0].usuarios_rechazados.clear();
            contract.elecciones[0].usuarios_pendientes.clear();
        

        }

        #[ink::test]
        fn test_iniciar_votacion_privado() 
        {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Crear el contrato con el administrador
            let mut contrato = SistemaElecciones::new();

            // Añadir una elección de prueba
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_pendientes: vec![],
                usuarios_rechazados: vec![],
                votacion_iniciada: false,
                fecha_inicio: 50,
                fecha_final: 150,
                resultados: None,
            });

            // Caso 1: No es administrador
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contrato.iniciar_votacion_privado(1),
                Err(String::from("No eres el administrador."))
            );

            // Caso 2: Elección no encontrada
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice); // Restaurar administrador
            assert_eq!(
                contrato.iniciar_votacion_privado(2),
                Err(String::from("No existe una elección con ese id."))
            );

            // Caso 3: Votación ya finalizó
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(200);
            assert_eq!(
                contrato.iniciar_votacion_privado(1),
                Err(String::from("La votación ya finalizó."))
            );

            // Caso 4: Votación ya inició
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(100); // Restaurar el timestamp del bloque
            contrato.elecciones[0].votacion_iniciada = true;
            assert_eq!(
                contrato.iniciar_votacion_privado(1),
                Err(String::from("La votación ya inició."))
            );

            // Caso 5: Todavía no es la fecha para la votación
            contrato.elecciones[0].votacion_iniciada = false;
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(30); // Cambiar el timestamp del bloque
            assert_eq!(
                contrato.iniciar_votacion_privado(1),
                Err(String::from("Todavía no es la fecha para la votación."))
            );

            // Caso 6: Se inició la votación exitosamente
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(100); // Restaurar el timestamp del bloque
            assert_eq!(
                contrato.iniciar_votacion_privado(1),
                Ok(String::from("Se inició la votación exitosamente."))
            );
            assert!(contrato.elecciones[0].votacion_iniciada);
        }

        #[ink::test]
        fn test_obtener_siguiente_usuario_pendiente_en_una_eleccion_privado_exito_VOTANTE() 
        {
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]); 
            let id_usuario = AccountId::from([2; 32]);

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Simular una elección válida con usuarios pendientes
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![(id_usuario, TIPO_DE_USUARIO::VOTANTE)],
                votacion_iniciada: false,
                fecha_inicio: 0, 
                fecha_final: 100, 
                resultados: None,
            });

            // Ejecutar el método para obtener el siguiente usuario pendiente en la elección
            let result_obtener = contrato.obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(1);

            // Verificar el resultado esperado
            assert!(result_obtener.is_ok(), "Error al obtener el siguiente usuario pendiente en la elección");

            // Ejecutar el método para obtener el siguiente usuario pendiente en la elección
            let result_obtener = contrato.obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(1);

            // Verificar el resultado esperado
            assert!(result_obtener.is_ok(), "Error al obtener el siguiente usuario pendiente en la elección");
        }

        #[ink::test]
        fn test_obtener_siguiente_usuario_pendiente_en_una_eleccion_privado_exito_CANDIDATO() 
        {
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]); 
            let id_usuario = AccountId::from([2; 32]);

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Simular una elección válida con usuarios pendientes
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![(id_usuario, TIPO_DE_USUARIO::CANDIDATO)],
                votacion_iniciada: false,
                fecha_inicio: 0, 
                fecha_final: 100, 
                resultados: None,
            });

            // Ejecutar el método para obtener el siguiente usuario pendiente en la elección
            let result_obtener = contrato.obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(1);

            // Verificar el resultado esperado
            assert!(result_obtener.is_ok(), "Error al obtener el siguiente usuario pendiente en la elección");

            // Ejecutar el método para obtener el siguiente usuario pendiente en la elección
            let result_obtener = contrato.obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(1);

            // Verificar el resultado esperado
            assert!(result_obtener.is_ok(), "Error al obtener el siguiente usuario pendiente en la elección");
        }


        #[ink::test]
        fn test_obtener_siguiente_usuario_pendiente_en_una_eleccion_privado_fallo_eleccion_inexistente()
        {
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]);
            let id_usuario = AccountId::from([2; 32]);

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Simular una elección válida con usuarios pendientes
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![(id_usuario, TIPO_DE_USUARIO::VOTANTE)],
                votacion_iniciada: false,
                fecha_inicio: 0, 
                fecha_final: 100, 
                resultados: None,
            });

            // Ejecutar el método para obtener el siguiente usuario pendiente en la elección
            let result_obtener = contrato.obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(10);

            // Verificar el resultado esperado
            assert!(result_obtener.is_err(), "Se esperaba un error al obtener el siguiente usuario pendiente en la elección (no se encontró elección)");
        }

        #[ink::test]
        fn test_obtener_siguiente_usuario_pendiente_en_una_eleccion_privado_fallo_sin_usuarios_pendientes() 
        {
            // Configuración inicial del contrato y del contexto de prueba
            let mut contrato = SistemaElecciones::new();
            let id_administrador = AccountId::from([1; 32]); // Ejemplo de ID de administrador

            // Simular un administrador registrado
            contrato.administrador = id_administrador;

            // Simular una elección válida sin usuarios pendientes
            contrato.elecciones.push(Eleccion {
                id: 1,
                candidatos: vec![],
                votantes: vec![],
                usuarios_rechazados: vec![],
                usuarios_pendientes: vec![],
                votacion_iniciada: false,
                fecha_inicio: 0, // Ajustar según la lógica de tu contrato
                fecha_final: 100, // Ajustar según la lógica de tu contrato
                resultados: None,
            });

            // Ejecutar el método para obtener el siguiente usuario pendiente en la elección
            let result_obtener = contrato.obtener_siguiente_usuario_pendiente_en_una_eleccion_privado(1);

            // Verificar el resultado esperado
            assert!(result_obtener.is_err(), "Se esperaba un error al obtener el siguiente usuario pendiente en la elección (ningún usuario pendiente)");
        }
        
    }
}    
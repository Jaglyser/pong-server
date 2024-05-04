use std::{net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket}, thread, time::{Duration, Instant}};

struct World {
    players: Vec<Renderable>,
    last_updated: Vec<Speed>
}

impl World {
    fn new() -> Self {
        World {
            players: Vec::new(),
            last_updated: Vec::new() 
        }
    }

    fn create_player(&mut self, source: SocketAddr) -> &Renderable {
        let player = if self.players.len() == 0 {
            Renderable { x: 20, y: 100, width: 20, height: 100, source }
        } else {
            Renderable { x: 760, y: 100, width: 20, height: 100, source }
        };
        let speed = Speed { dx: 0, dy: 0, last_update: Instant::now(), source };
        self.players.push(player);
        self.last_updated.push(speed);

        return self.players.last().unwrap();
    }

    fn get_last_updated(&self, renderable: &Renderable) -> Instant {
        self.last_updated.iter()
            .find(|speed| speed.source == renderable.source)
            .map(|speed| speed.last_update)
            .unwrap()
    }

    fn update_time(&mut self, renderable: &Renderable) {
        self.last_updated.iter_mut()
            .find(|speed| speed.source == renderable.source)
            .map(|speed| speed.last_update = Instant::now());
    }
}

struct Renderable {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    source: SocketAddr
}

struct Speed {
    dy: i32,
    dx: i32,
    last_update: Instant,
    source: SocketAddr
}

impl ToString for Renderable  {
    fn to_string(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.width, self.height)
    }
    
}

struct BallSystem {
    ball: Renderable,
    velocity: Speed,
}


impl BallSystem {
    fn new() -> Self {
        BallSystem {
            ball: Renderable { x: 400, y: 300, width: 20, height: 20, source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0) },
            velocity: Speed { dx: 1, dy: 1, last_update: Instant::now(), source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0) }
        }
    }

    fn update(&mut self) {
        self.ball.x += self.velocity.dx;
    }

    fn player_collision(&self, player: &Renderable) -> bool {
        self.ball.x < player.x + player.width &&
            self.ball.x + self.ball.width > player.x &&
            self.ball.y < player.y + player.height &&
            self.ball.y + self.ball.height > player.y
    }

    fn goal(&self) -> bool {
        self.ball.x < 0 || self.ball.x > 800
    }

    fn wall_collision(&self) -> bool {
        self.ball.y < 0 || self.ball.y > 600
    }

    fn bounce(&mut self) {
        self.velocity.dx = -1*self.velocity.dx;
    }

    fn new_ball (&mut self) {
        self.ball.x = 400;
        self.ball.y = 300;
    }

}

struct NetworkSystem {
    socket: UdpSocket,
    buf: [u8; 1024],
}

impl NetworkSystem {
    fn new() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:8080")
            .expect("Failed to bind to address");

        socket.set_nonblocking(true)
            .expect("Failed to set non-blocking");

        println!("Server listening on 127.0.0.1:8080");

        NetworkSystem {
            socket,
            buf: [0; 1024],
        }
    }

    fn receive(&mut self) -> Result<(usize, SocketAddr), std::io::Error> {
        self.socket.recv_from(&mut self.buf)
    }

    fn parse_request(&self, size: usize) -> &str {
        std::str::from_utf8(&self.buf[..size]).unwrap()
    }
}

struct ControlSystem;

impl ControlSystem {
    fn new() -> Self {
        ControlSystem
    }


}

fn main() {
    let mut world = World::new();
    let mut network_system = NetworkSystem::new();
    let mut ball_system = BallSystem::new();
    let mut control_system = ControlSystem::new();

    loop {
        let (size, source) = match network_system.receive() {
            Ok((size, source)) => (size, source),
            Err(_) => continue,
        };
            
        let request = network_system.parse_request(size);
        //println!("Received request: {} from {}", request, source);

        if request == "join" && world.players.len() < 3 {
            let player = world.create_player(source);
            let response = format!("{} {} {} {}", player.x, player.y, player.width, player.height);
            network_system.socket.send_to(response.as_bytes(), source).expect("Failed to send response");

            println!("Player joined! Total players: {}", world.players.len());
            //world.players.push(player);
        } else {


            let renderable = world.players.iter_mut()
                .find(|player| player.source == source)
                .map(|player| {
                    let parts: Vec<&str> = request.split_whitespace().collect();

                    let y = parts[1].parse::<i32>().unwrap();

                    player.y = y;
                    player.width = parts[2].parse().unwrap();
                    player.height = parts[3].parse().unwrap();
                });

            let player_state =  world.players.iter()
                .filter(|player| player.source != source)
                .filter(|player| player.width != player.height)
                .map(|player| player.to_string())
                .collect::<Vec<String>>()
                .join("");


            let ball_state =  ball_system.ball.to_string();

            let state = [player_state, ball_state].join(" ");

            network_system.socket.send_to(state.as_bytes(), source).expect("Failed to send response");
        }
        if world.players.len() == 2 {
            ball_system.update();
            world.players.iter().find(|player| ball_system.player_collision(player))
                .map(|_| ball_system.bounce());

            ball_system.goal().then(|| ball_system.new_ball());
        }
    }
}



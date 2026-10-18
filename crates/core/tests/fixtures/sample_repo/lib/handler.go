package lib

type Server struct{}

func (s *Server) Serve() {
	s.ping()
}

func (s *Server) ping() {}

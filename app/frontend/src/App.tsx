import { BrowserRouter, Route, Routes } from 'react-router-dom'
import UserListPage from './pages/UserListPage'
import UserDetail from './pages/UserDetail'

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<UserListPage />} />
        <Route path="/users/:id" element={<UserDetail />} />
      </Routes>
    </BrowserRouter>
  )
}

export default App

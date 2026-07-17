import { BrowserRouter, Route, Routes } from 'react-router-dom'
import { AuthProvider } from './auth/AuthContext'
import ProtectedRoute from './auth/ProtectedRoute'
import AuthPage from './pages/AuthPage'
import UserListPage from './pages/UserListPage'
import UserDetail from './pages/UserDetail'

function App() {
  return (
    <BrowserRouter>
      <AuthProvider><Routes>
        <Route path="/login" element={<AuthPage mode="login" />} />
        <Route path="/register" element={<AuthPage mode="register" />} />
        <Route element={<ProtectedRoute />}>
          <Route path="/" element={<UserListPage />} />
          <Route path="/users/:id" element={<UserDetail />} />
        </Route>
      </Routes></AuthProvider>
    </BrowserRouter>
  )
}

export default App

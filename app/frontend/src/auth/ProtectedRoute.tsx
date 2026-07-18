import { Navigate, Outlet, useLocation } from 'react-router-dom'
import { useAuth } from './AuthContext'

export default function ProtectedRoute() {
  const { account, loading } = useAuth()
  const location = useLocation()
  if (loading) return <main className="auth-loading" role="status">正在恢复登录状态…</main>
  if (!account) return <Navigate to="/login" replace state={{ from: location.pathname }} />
  return <Outlet />
}

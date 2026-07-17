/* oxlint-disable react/only-export-components -- provider and hook intentionally share one private context */
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import * as authApi from '../api/auth'
import { setAccessToken, setRefreshHandler } from '../api/client'

interface AuthContextValue {
  account: authApi.Account | null
  loading: boolean
  login(username: string, password: string): Promise<void>
  register(username: string, password: string): Promise<void>
  logout(): Promise<void>
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [account, setAccount] = useState<authApi.Account | null>(null)
  const [loading, setLoading] = useState(true)
  const refreshInFlight = useRef<Promise<boolean> | null>(null)

  const apply = useCallback((response: authApi.AuthResponse) => {
    setAccessToken(response.access_token)
    setAccount(response.account)
  }, [])

  const refresh = useCallback(async () => {
    if (refreshInFlight.current) return refreshInFlight.current
    refreshInFlight.current = (async () => {
      try { apply(await authApi.refresh()); return true }
      catch { setAccessToken(null); setAccount(null); return false }
      finally { refreshInFlight.current = null }
    })()
    return refreshInFlight.current
  }, [apply])

  useEffect(() => {
    setRefreshHandler(refresh)
    void refresh().finally(() => setLoading(false))
    return () => setRefreshHandler(null)
  }, [refresh])

  const value = useMemo<AuthContextValue>(() => ({
    account, loading,
    login: async (username, password) => apply(await authApi.login(username, password)),
    register: async (username, password) => apply(await authApi.register(username, password)),
    logout: async () => { try { await authApi.logout() } finally { setAccessToken(null); setAccount(null) } },
  }), [account, loading, apply])

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const value = useContext(AuthContext)
  if (!value) throw new Error('useAuth must be used within AuthProvider')
  return value
}

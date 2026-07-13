import { useCallback, useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { getUser, refreshUser } from '../api/users'
import ProfileCard from '../components/ProfileCard'
import type { GithubUser } from '../types/user'
import './UserDetail.css'

/** 用户详情页：ProfileCard 展示 + 刷新按钮（POST /api/users/:id/refresh） */
export default function UserDetail() {
  const { id } = useParams<{ id: string }>()
  const [user, setUser] = useState<GithubUser | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!id) return
    let cancelled = false
    setLoading(true)
    setError(null)
    getUser(id)
      .then((data) => {
        if (!cancelled) setUser(data)
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(err instanceof Error ? err.message : '加载用户详情失败')
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [id])

  const handleRefresh = useCallback(async () => {
    if (!id || refreshing) return
    setRefreshing(true)
    setError(null)
    try {
      const data = await refreshUser(id)
      setUser(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : '刷新失败，请稍后重试')
    } finally {
      setRefreshing(false)
    }
  }, [id, refreshing])

  return (
    <main className="user-detail">
      <div className="user-detail__toolbar">
        <Link className="user-detail__back" to="/">
          ← 返回列表
        </Link>
        <button
          type="button"
          className="user-detail__refresh"
          onClick={handleRefresh}
          disabled={loading || refreshing || !user}
        >
          {refreshing ? '刷新中…' : '刷新'}
        </button>
      </div>

      {error && (
        <p className="user-detail__error" role="alert">
          {error}
        </p>
      )}

      {loading ? (
        <p className="user-detail__status">加载中…</p>
      ) : user ? (
        <ProfileCard user={user} />
      ) : (
        !error && <p className="user-detail__status">未找到该用户</p>
      )}
    </main>
  )
}

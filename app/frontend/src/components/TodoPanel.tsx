import { useCallback, useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { createTodo, deleteTodo, listTodos, updateTodo } from '../api/todos'
import type { Todo } from '../types/todo'
import './TodoPanel.css'

const PAGE_SIZE = 10

interface TodoPanelProps {
  userId: string
}

export default function TodoPanel({ userId }: TodoPanelProps) {
  const [items, setItems] = useState<Todo[]>([])
  const [total, setTotal] = useState(0)
  const [page, setPage] = useState(1)
  const [loading, setLoading] = useState(true)
  const [submitting, setSubmitting] = useState(false)
  const [busyId, setBusyId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editTitle, setEditTitle] = useState('')
  const [editDescription, setEditDescription] = useState('')

  const load = useCallback(async (targetPage: number) => {
    setLoading(true)
    setError(null)
    try {
      const result = await listTodos(userId, targetPage, PAGE_SIZE)
      setItems(result.items)
      setTotal(result.total)
      setPage(result.page)
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载 Todo 失败')
    } finally {
      setLoading(false)
    }
  }, [userId])

  useEffect(() => {
    void load(1)
  }, [load])

  const handleCreate = async (event: FormEvent) => {
    event.preventDefault()
    if (!title.trim() || submitting) return
    setSubmitting(true)
    setError(null)
    try {
      await createTodo(userId, {
        title,
        description: description.trim() || null,
      })
      setTitle('')
      setDescription('')
      await load(1)
    } catch (err) {
      setError(err instanceof Error ? err.message : '创建 Todo 失败')
    } finally {
      setSubmitting(false)
    }
  }

  const handleToggle = async (todo: Todo) => {
    setBusyId(todo.id)
    setError(null)
    try {
      const updated = await updateTodo(userId, todo.id, { completed: !todo.completed })
      setItems((current) => current.map((item) => item.id === updated.id ? updated : item))
    } catch (err) {
      setError(err instanceof Error ? err.message : '更新 Todo 失败')
    } finally {
      setBusyId(null)
    }
  }

  const startEdit = (todo: Todo) => {
    setEditingId(todo.id)
    setEditTitle(todo.title)
    setEditDescription(todo.description ?? '')
  }

  const saveEdit = async (todoId: string) => {
    if (!editTitle.trim()) return
    setBusyId(todoId)
    setError(null)
    try {
      const updated = await updateTodo(userId, todoId, {
        title: editTitle,
        description: editDescription.trim() || null,
      })
      setItems((current) => current.map((item) => item.id === updated.id ? updated : item))
      setEditingId(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存 Todo 失败')
    } finally {
      setBusyId(null)
    }
  }

  const handleDelete = async (todoId: string) => {
    setBusyId(todoId)
    setError(null)
    try {
      await deleteTodo(userId, todoId)
      const nextPage = items.length === 1 && page > 1 ? page - 1 : page
      await load(nextPage)
    } catch (err) {
      setError(err instanceof Error ? err.message : '删除 Todo 失败')
    } finally {
      setBusyId(null)
    }
  }

  const pages = Math.max(1, Math.ceil(total / PAGE_SIZE))

  return (
    <section className="todo-panel" aria-labelledby="todo-panel-title">
      <div className="todo-panel__heading">
        <div>
          <p className="todo-panel__eyebrow">Go Todo Service</p>
          <h2 id="todo-panel-title">Todo List</h2>
        </div>
        <span className="todo-panel__count">{total} 项</span>
      </div>

      <form className="todo-panel__create" onSubmit={handleCreate}>
        <label>
          标题
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="例如：整理 GitHub Profile"
            maxLength={200}
            required
            disabled={submitting}
          />
        </label>
        <label>
          描述（可选）
          <textarea
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            placeholder="补充 Todo 的详细说明"
            maxLength={2000}
            disabled={submitting}
          />
        </label>
        <button type="submit" disabled={submitting || !title.trim()}>
          {submitting ? '创建中…' : '添加 Todo'}
        </button>
      </form>

      {error && <p className="todo-panel__error" role="alert">{error}</p>}

      {loading ? (
        <p className="todo-panel__status">加载 Todo 中…</p>
      ) : !error && items.length === 0 ? (
        <p className="todo-panel__status">还没有 Todo，先添加一项吧。</p>
      ) : (
        <ul className="todo-panel__list">
          {items.map((todo) => (
            <li className={todo.completed ? 'todo-panel__item todo-panel__item--done' : 'todo-panel__item'} key={todo.id}>
              {editingId === todo.id ? (
                <div className="todo-panel__editor">
                  <label>
                    编辑标题
                    <input value={editTitle} onChange={(event) => setEditTitle(event.target.value)} maxLength={200} />
                  </label>
                  <label>
                    编辑描述
                    <textarea value={editDescription} onChange={(event) => setEditDescription(event.target.value)} maxLength={2000} />
                  </label>
                  <div className="todo-panel__actions">
                    <button type="button" onClick={() => void saveEdit(todo.id)} disabled={busyId === todo.id || !editTitle.trim()}>保存</button>
                    <button type="button" className="secondary" onClick={() => setEditingId(null)} disabled={busyId === todo.id}>取消</button>
                  </div>
                </div>
              ) : (
                <>
                  <label className="todo-panel__summary">
                    <input type="checkbox" checked={todo.completed} onChange={() => void handleToggle(todo)} disabled={busyId === todo.id} />
                    <span>
                      <strong>{todo.title}</strong>
                      {todo.description && <small>{todo.description}</small>}
                    </span>
                  </label>
                  <div className="todo-panel__actions">
                    <button type="button" className="secondary" onClick={() => startEdit(todo)} disabled={busyId === todo.id}>编辑</button>
                    <button type="button" className="danger" onClick={() => void handleDelete(todo.id)} disabled={busyId === todo.id}>删除</button>
                  </div>
                </>
              )}
            </li>
          ))}
        </ul>
      )}

      {pages > 1 && (
        <nav className="todo-panel__pagination" aria-label="Todo 分页">
          <button type="button" onClick={() => void load(page - 1)} disabled={loading || page <= 1}>上一页</button>
          <span>第 {page} / {pages} 页</span>
          <button type="button" onClick={() => void load(page + 1)} disabled={loading || page >= pages}>下一页</button>
        </nav>
      )}
    </section>
  )
}

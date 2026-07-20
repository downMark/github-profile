export interface Todo {
  id: string
  github_user_id: string
  title: string
  description: string | null
  completed: boolean
  created_at: string
  updated_at: string
}

export interface TodoListResult {
  items: Todo[]
  total: number
  page: number
  limit: number
}

export interface TodoEventAudit {
  event_id: string
  schema_version: number
  event_type: string
  occurred_at: string
  processed_at: string
  environment: string
  github_user_id: string
  todo_id: string
  todo: Todo | null
}

export interface TodoEventAuditListResult {
  items: TodoEventAudit[]
  total: number
  page: number
  limit: number
}

export interface CreateTodoInput {
  title: string
  description?: string | null
}

export interface UpdateTodoInput {
  title?: string
  description?: string | null
  completed?: boolean
}

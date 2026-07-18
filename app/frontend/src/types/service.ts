export type ServiceName = 'auth' | 'profile' | 'todo'

export interface ServiceMockResponse {
  service: ServiceName
  status: 'ok'
  message: string
  environment: string
  revision: string
}

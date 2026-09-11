export class Company {
  static init(name: string, mission: string, stateDirectory: string, dbPath: string): Promise<Company>;
  static initWithToken(name: string, mission: string, stateDirectory: string, dbPath: string, token: string, targetCompany?: string): Promise<Company>;
  static signup(dbPath: string, email: string, pass: string): Promise<string>;
  static signin(dbPath: string, email: string, pass: string): Promise<string>;
  static getCompanies(dbPath: string, token: string): Promise<string>;
  
  deleteCompany(id: string): Promise<void>;
  setPacing(requestsPerMinute: number, workingHoursStart?: string, workingHoursEnd?: string): Promise<void>;
  configureInference(configJson: string): Promise<void>;
  getCompanyName(): Promise<string | null>;
  foundCompany(name: string): Promise<void>;
  addAgent(agentJson: string): Promise<void>;
  updateAgent(id: string, agentJson: string): Promise<void>;
  removeAgent(id: string): Promise<void>;
  getAgents(): Promise<string>;
  getProjects(): Promise<string>;
  addTool(toolJson: string): Promise<void>;
  getTools(): Promise<string>;
  removeTool(name: string): Promise<void>;
  addSharedFile(fileJson: string): Promise<void>;
  getSharedTree(): Promise<string>;
  getAgentMemoryTree(agentId: string): Promise<string>;
  getAgentContext(agentId: string): Promise<string>;
  queueMessage(agentId: string, message: string): Promise<void>;
  registerToolExecutor(executor: (agentId: string, toolName: string, argsJson: string) => string): void;
  start(): Promise<void>;
}

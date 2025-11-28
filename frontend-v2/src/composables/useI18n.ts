import { ref } from 'vue'

const currentLocale = ref<'en' | 'zh-CN'>(navigator.language === 'zh-CN' ? 'zh-CN' : 'en')

const dictionary = {
    en: {
        'SELECT_PROJECT': 'SELECT PROJECT',
        'SEARCH_TASKS': 'SEARCH TASKS...',
        'TASKS': 'TASKS',
        'ONLINE': 'ONLINE',
        'OFFLINE': 'OFFLINE',
        'NEW_TASK': 'NEW TASK',
        'TASK_NAME': 'TASK NAME',
        'PARENT_TASK': 'PARENT TASK',
        'PRIORITY': 'PRIORITY',
        'DESCRIPTION': 'DESCRIPTION',
        'CANCEL': 'CANCEL',
        'CREATE': 'CREATE',
        'SPAWN_SUBTASK': 'SPAWN SUBTASK',
        'DELETE': 'DELETE',
        'DELETE_CONFIRM': 'Delete task "{name}"?',
        'AWAITING_SELECTION': 'AWAITING SELECTION',
        'MISSION_PARAMETERS': 'TASK PARAMETERS',
        'NO_SPEC': '// No specifications defined...',
        'ENTER_NAME': 'Enter task name...',
        'ENTER_DESC': 'Enter task description (Markdown supported)...',
        'NONE': 'None',
        'CRITICAL': 'Critical',
        'HIGH': 'High',
        'MEDIUM': 'Medium',
        'LOW': 'Low',
        'SYSTEM_LOGS': 'TASK EVENTS',
        'LIVE_FEED': 'LIVE FEED',
        'ENTRIES': 'ENTRIES',
        'ADD_NOTE': 'ADD_NOTE...',
        'NO_DATA_STREAM': 'NO_DATA_STREAM',
        'DELETE_LOG': 'Delete Event',
        'DELETE_LOG_CONFIRM': 'Delete this event?',
        'TYPE_DECISION': 'Decision',
        'TYPE_BLOCKER': 'Blocker',
        'TYPE_MILESTONE': 'Milestone',
        'TYPE_NOTE': 'Note',
        'START': 'START',
        'STOP': 'STOP',
        'DONE': 'DONE',
        'RESTART': 'RESTART',
        'MARK_TODO': 'MARK TODO',
        'MARK_DONE': 'MARK DONE',
        'DELETE_TASK_CONFIRM': 'Delete task "{name}"?',
        'TOTAL_TASKS': 'TASKS: {count}'
    },
    'zh-CN': {
        'SELECT_PROJECT': '选择项目',
        'SEARCH_TASKS': '搜索任务...',
        'TASKS': '个任务',
        'ONLINE': '在线',
        'OFFLINE': '离线',
        'NEW_TASK': '新建任务',
        'TASK_NAME': '任务名称',
        'PARENT_TASK': '父任务',
        'PRIORITY': '优先级',
        'DESCRIPTION': '任务描述',
        'CANCEL': '取消',
        'CREATE': '创建',
        'SPAWN_SUBTASK': '添加子任务',
        'DELETE': '删除',
        'DELETE_CONFIRM': '确认删除任务 "{name}"？',
        'AWAITING_SELECTION': '等待选择',
        'MISSION_PARAMETERS': '任务参数',
        'NO_SPEC': '// 未定义详细描述...',
        'ENTER_NAME': '输入任务名称...',
        'ENTER_DESC': '输入任务描述 (支持 Markdown)...',
        'NONE': '无',
        'CRITICAL': '紧急',
        'HIGH': '高',
        'MEDIUM': '中',
        'LOW': '低',
        'SYSTEM_LOGS': '任务事件',
        'LIVE_FEED': '实时动态',
        'ENTRIES': '条记录',
        'ADD_NOTE': '添加笔记...',
        'NO_DATA_STREAM': '暂无数据流',
        'DELETE_LOG': '删除事件',
        'DELETE_LOG_CONFIRM': '确认删除这个事件？',
        'TYPE_DECISION': '决策',
        'TYPE_BLOCKER': '阻碍',
        'TYPE_MILESTONE': '里程碑',
        'TYPE_NOTE': '笔记',
        'START': '开始',
        'STOP': '停止',
        'DONE': '完成',
        'RESTART': '重新开始',
        'MARK_TODO': '标记为待办',
        'MARK_DONE': '标记为完成',
        'DELETE_TASK_CONFIRM': '确认删除任务 "{name}"？',
        'TOTAL_TASKS': '任务数: {count}'
    }
}

export function useI18n() {
    const t = (key: keyof typeof dictionary['en'], params?: Record<string, string>) => {
        let text = dictionary[currentLocale.value][key] || key
        if (params) {
            Object.entries(params).forEach(([k, v]) => {
                text = text.replace(`{${k}}`, v)
            })
        }
        return text
    }

    const toggleLocale = () => {
        currentLocale.value = currentLocale.value === 'en' ? 'zh-CN' : 'en'
    }

    return {
        currentLocale,
        t,
        toggleLocale
    }
}

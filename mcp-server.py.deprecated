#!/usr/bin/env python3
"""
Intent-Engine MCP Server
Wraps intent-engine CLI to provide Model Context Protocol interface
"""

import json
import subprocess
import sys
from typing import Any, Dict


def call_intent_engine(args: list[str], stdin_data: str = None) -> Dict[str, Any]:
    """Call intent-engine CLI and return JSON output"""
    try:
        cmd = ["intent-engine"] + args
        result = subprocess.run(
            cmd,
            input=stdin_data,
            capture_output=True,
            text=True,
            check=True
        )
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        # Try to parse error as JSON
        try:
            error_data = json.loads(e.stderr)
            raise Exception(error_data.get("error", str(e)))
        except json.JSONDecodeError:
            raise Exception(f"CLI error: {e.stderr}")
    except json.JSONDecodeError as e:
        raise Exception(f"Invalid JSON from CLI: {e}")


def handle_task_add(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_add tool call"""
    args = ["task", "add", "--name", params["name"]]

    stdin_data = None
    if "spec" in params:
        args.append("--spec-stdin")
        stdin_data = params["spec"]

    if "parent_id" in params:
        args.extend(["--parent", str(params["parent_id"])])

    return call_intent_engine(args, stdin_data)


def handle_task_start(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_start tool call"""
    args = ["task", "start", str(params["task_id"])]

    if params.get("with_events", True):
        args.append("--with-events")

    return call_intent_engine(args)


def handle_task_pick_next(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_pick_next tool call"""
    args = ["task", "pick-next"]

    if "max_count" in params:
        args.extend(["--max-count", str(params["max_count"])])

    if "capacity" in params:
        args.extend(["--capacity", str(params["capacity"])])

    return call_intent_engine(args)


def handle_task_spawn_subtask(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_spawn_subtask tool call"""
    args = ["task", "spawn-subtask", "--name", params["name"]]

    stdin_data = None
    if "spec" in params:
        args.append("--spec-stdin")
        stdin_data = params["spec"]

    return call_intent_engine(args, stdin_data)


def handle_task_switch(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_switch tool call"""
    args = ["task", "switch", str(params["task_id"])]
    return call_intent_engine(args)


def handle_task_done(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_done tool call"""
    args = ["task", "done", str(params["task_id"])]
    return call_intent_engine(args)


def handle_task_update(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_update tool call"""
    args = ["task", "update", str(params["task_id"])]

    if "name" in params:
        args.extend(["--name", params["name"]])

    if "status" in params:
        args.extend(["--status", params["status"]])

    if "complexity" in params:
        args.extend(["--complexity", str(params["complexity"])])

    if "priority" in params:
        args.extend(["--priority", str(params["priority"])])

    if "parent_id" in params:
        args.extend(["--parent", str(params["parent_id"])])

    stdin_data = None
    if "spec" in params:
        args.append("--spec-stdin")
        stdin_data = params["spec"]

    return call_intent_engine(args, stdin_data)


def handle_task_find(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_find tool call"""
    args = ["task", "find"]

    if "status" in params:
        args.extend(["--status", params["status"]])

    if "parent" in params:
        args.extend(["--parent", params["parent"]])

    return call_intent_engine(args)


def handle_task_get(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle task_get tool call"""
    args = ["task", "get", str(params["task_id"])]

    if params.get("with_events", False):
        args.append("--with-events")

    return call_intent_engine(args)


def handle_event_add(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle event_add tool call"""
    args = [
        "event", "add",
        "--task-id", str(params["task_id"]),
        "--type", params["event_type"],
        "--data-stdin"
    ]

    return call_intent_engine(args, params["data"])


def handle_event_list(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle event_list tool call"""
    args = ["event", "list", "--task-id", str(params["task_id"])]

    if "limit" in params:
        args.extend(["--limit", str(params["limit"])])

    return call_intent_engine(args)


def handle_current_task_get(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle current_task_get tool call"""
    args = ["current"]
    return call_intent_engine(args)


def handle_report_generate(params: Dict[str, Any]) -> Dict[str, Any]:
    """Handle report_generate tool call"""
    args = ["report"]

    if params.get("summary_only", True):
        args.append("--summary-only")

    if "since" in params:
        args.extend(["--since", params["since"]])

    if "status" in params:
        args.extend(["--status", params["status"]])

    if "filter_name" in params:
        args.extend(["--filter-name", params["filter_name"]])

    if "filter_spec" in params:
        args.extend(["--filter-spec", params["filter_spec"]])

    return call_intent_engine(args)


# Tool handlers mapping
TOOL_HANDLERS = {
    "task_add": handle_task_add,
    "task_start": handle_task_start,
    "task_pick_next": handle_task_pick_next,
    "task_spawn_subtask": handle_task_spawn_subtask,
    "task_switch": handle_task_switch,
    "task_done": handle_task_done,
    "task_update": handle_task_update,
    "task_find": handle_task_find,
    "task_get": handle_task_get,
    "event_add": handle_event_add,
    "event_list": handle_event_list,
    "current_task_get": handle_current_task_get,
    "report_generate": handle_report_generate,
}


def handle_request(request: Dict[str, Any]) -> Dict[str, Any]:
    """Handle MCP request"""
    method = request.get("method")
    params = request.get("params", {})

    if method == "tools/list":
        # Return available tools
        with open("mcp-server.json") as f:
            config = json.load(f)
        return {"tools": config["tools"]}

    elif method == "tools/call":
        tool_name = params.get("name")
        tool_params = params.get("arguments", {})

        if tool_name not in TOOL_HANDLERS:
            raise Exception(f"Unknown tool: {tool_name}")

        result = TOOL_HANDLERS[tool_name](tool_params)
        return {"content": [{"type": "text", "text": json.dumps(result, indent=2)}]}

    else:
        raise Exception(f"Unknown method: {method}")


def main():
    """Main MCP server loop"""
    while True:
        try:
            # Read JSON-RPC request from stdin
            line = sys.stdin.readline()
            if not line:
                break

            request = json.loads(line)

            try:
                result = handle_request(request)
                response = {
                    "jsonrpc": "2.0",
                    "id": request.get("id"),
                    "result": result
                }
            except Exception as e:
                response = {
                    "jsonrpc": "2.0",
                    "id": request.get("id"),
                    "error": {
                        "code": -32000,
                        "message": str(e)
                    }
                }

            # Write response to stdout
            print(json.dumps(response), flush=True)

        except json.JSONDecodeError as e:
            error_response = {
                "jsonrpc": "2.0",
                "id": None,
                "error": {
                    "code": -32700,
                    "message": f"Parse error: {e}"
                }
            }
            print(json.dumps(error_response), flush=True)
        except Exception as e:
            error_response = {
                "jsonrpc": "2.0",
                "id": None,
                "error": {
                    "code": -32603,
                    "message": f"Internal error: {e}"
                }
            }
            print(json.dumps(error_response), flush=True)


if __name__ == "__main__":
    main()

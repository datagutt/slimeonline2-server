#!/usr/bin/env python3
"""
Extract room data from GameMaker GMX files for Slime Online 2.
This extracts spawn points, warps, and collectible info for the server.
"""

import os
import re
import json
import xml.etree.ElementTree as ET
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import List, Dict, Optional, Any

# Room directory - use absolute path
GMX_BASE = Path("/mnt/c/Users/datag/Documents/slime/slime2_decompile.gmx")
GMX_ROOMS_DIR = GMX_BASE / "rooms"


@dataclass
class SpawnPoint:
    x: int
    y: int


@dataclass
class Warp:
    x: int
    y: int
    warp_type: str  # warp_vert, warp_hor, warp_button, etc.
    next_room: Optional[int]
    new_x: Optional[int]
    new_y: Optional[int]
    code: str  # Raw code for any extra parsing


@dataclass
class Instance:
    obj_name: str
    x: int
    y: int
    code: str
    instance_id: int


@dataclass
class RoomData:
    name: str
    room_index: int  # Based on position in project
    width: int
    height: int
    has_collectibles: bool  # rm_points = 1
    spawn_points: List[SpawnPoint]
    warps: List[Warp]
    npcs: List[Instance]
    shops: List[Instance]
    special_objects: List[Instance]  # banks, mailboxes, etc.


def parse_code_vars(code: str) -> Dict[str, Any]:
    """Parse instance code to extract variable assignments."""
    vars = {}
    if not code:
        return vars

    # Decode XML entities
    code = code.replace("&#13;", "\r").replace("&#10;", "\n")

    # Match patterns like: var_name = value
    for match in re.finditer(r"(\w+)\s*=\s*([^\r\n]+)", code):
        name = match.group(1).strip()
        value = match.group(2).strip()

        # Try to parse as number
        try:
            if "." in value:
                vars[name] = float(value)
            else:
                vars[name] = int(value)
        except ValueError:
            vars[name] = value

    return vars


def parse_room_file(filepath: Path) -> Optional[RoomData]:
    """Parse a single GMX room file."""
    try:
        tree = ET.parse(filepath)
        root = tree.getroot()
    except Exception as e:
        print(f"Error parsing {filepath}: {e}")
        return None

    room_name = filepath.stem.replace(".room", "")

    # Get dimensions
    width_elem = root.find("width")
    height_elem = root.find("height")
    width = int(width_elem.text) if width_elem is not None and width_elem.text else 0
    height = (
        int(height_elem.text) if height_elem is not None and height_elem.text else 0
    )

    # Check for rm_points (collectibles allowed)
    code_elem = root.find("code")
    room_code = code_elem.text if code_elem is not None and code_elem.text else ""
    has_collectibles = "rm_points = 1" in room_code

    spawn_points = []
    warps = []
    npcs = []
    shops = []
    special_objects = []

    instances = root.find("instances")
    if instances is None:
        return RoomData(
            name=room_name,
            room_index=0,
            width=width,
            height=height,
            has_collectibles=has_collectibles,
            spawn_points=[],
            warps=[],
            npcs=[],
            shops=[],
            special_objects=[],
        )

    for inst in instances.findall("instance"):
        obj_name = inst.get("objName", "")
        x = int(float(inst.get("x", 0)))
        y = int(float(inst.get("y", 0)))
        code = inst.get("code", "")
        inst_id = int(inst.get("id", 0))

        code_vars = parse_code_vars(code)

        # Spawn points (slimepoints)
        if obj_name == "obj_slimepoint":
            spawn_points.append(SpawnPoint(x=x, y=y))

        # Warps
        elif obj_name.startswith("warp_"):
            warp = Warp(
                x=x,
                y=y,
                warp_type=obj_name,
                next_room=code_vars.get("next_room"),
                new_x=code_vars.get("new_x"),
                new_y=code_vars.get("new_y"),
                code=code,
            )
            warps.append(warp)

        # NPCs
        elif obj_name.startswith("NPC_"):
            npcs.append(
                Instance(obj_name=obj_name, x=x, y=y, code=code, instance_id=inst_id)
            )

        # Shops
        elif obj_name in ["obj_shop_item", "obj_shop_call_item", "obj_sell_sign"]:
            shops.append(
                Instance(obj_name=obj_name, x=x, y=y, code=code, instance_id=inst_id)
            )

        # Special objects
        elif obj_name in [
            "obj_bank",
            "obj_mailbox",
            "obj_storage_box",
            "obj_save_bg",
            "obj_clock_stand",
            "obj_warpcenter",
            "obj_clan_machine",
            "obj_post_office",
            "obj_gum_machine",
            "obj_soda_machine",
            "obj_upgrader",
            "Planting_Field",
            "Building_Spot",
            "obj_drill",
            "obj_race_machine",
            "obj_race_start_ver",
            "obj_race_start_hor",
            "obj_race_end_ver",
            "obj_race_end_hor",
            "obj_teleporter",
            "obj_music_changer",
            "obj_billboard",
            "obj_combinator",
        ]:
            special_objects.append(
                Instance(obj_name=obj_name, x=x, y=y, code=code, instance_id=inst_id)
            )

    return RoomData(
        name=room_name,
        room_index=0,  # Will be set later
        width=width,
        height=height,
        has_collectibles=has_collectibles,
        spawn_points=spawn_points,
        warps=warps,
        npcs=npcs,
        shops=shops,
        special_objects=special_objects,
    )


def get_room_order() -> List[str]:
    """Get room order from project file."""
    project_file = GMX_BASE / "slime2_decompile.project.gmx"

    try:
        tree = ET.parse(project_file)
        root = tree.getroot()
    except Exception as e:
        print(f"Error parsing project file: {e}")
        return []

    rooms_elem = root.find(".//rooms[@name='rooms']")
    if rooms_elem is None:
        return []

    rooms = []
    for room in rooms_elem.findall("room"):
        if room.text:
            room_name = room.text.replace("rooms\\", "")
            rooms.append(room_name)

    return rooms


def main():
    # Get room order
    room_order = get_room_order()
    room_to_index = {name: idx for idx, name in enumerate(room_order)}

    print(f"Found {len(room_order)} rooms in project")

    all_rooms = {}

    # Parse each room file
    for room_file in sorted(GMX_ROOMS_DIR.glob("*.room.gmx")):
        room_data = parse_room_file(room_file)
        if room_data:
            # Set room index
            room_data.room_index = room_to_index.get(room_data.name, -1)
            all_rooms[room_data.name] = room_data

    # Print summary
    print("\n=== ROOM SUMMARY ===\n")

    # Rooms with spawn points
    rooms_with_spawns = [r for r in all_rooms.values() if r.spawn_points]
    print(f"Rooms with spawn points: {len(rooms_with_spawns)}")
    for room in sorted(rooms_with_spawns, key=lambda x: x.room_index):
        print(
            f"  [{room.room_index:3d}] {room.name}: {len(room.spawn_points)} spawn points"
        )

    # Rooms with collectibles enabled
    print(f"\nRooms with collectibles enabled (rm_points=1):")
    collectible_rooms = [r for r in all_rooms.values() if r.has_collectibles]
    for room in sorted(collectible_rooms, key=lambda x: x.room_index):
        print(f"  [{room.room_index:3d}] {room.name} ({room.width}x{room.height})")

    # Warp connections
    print(f"\n=== WARP CONNECTIONS ===\n")
    for room in sorted(all_rooms.values(), key=lambda x: x.room_index):
        if room.warps:
            print(f"[{room.room_index:3d}] {room.name}:")
            for warp in room.warps:
                if warp.next_room is not None and isinstance(warp.next_room, int):
                    target_name = (
                        room_order[warp.next_room]
                        if 0 <= warp.next_room < len(room_order)
                        else f"room_{warp.next_room}"
                    )
                    print(
                        f"  {warp.warp_type} at ({warp.x}, {warp.y}) -> [{warp.next_room}] {target_name} at ({warp.new_x}, {warp.new_y})"
                    )

    # Export to JSON for server use
    output = {"rooms": {}, "room_order": room_order}

    for name, room in all_rooms.items():
        output["rooms"][name] = {
            "index": room.room_index,
            "width": room.width,
            "height": room.height,
            "has_collectibles": room.has_collectibles,
            "spawn_points": [asdict(sp) for sp in room.spawn_points],
            "warps": [
                {
                    "x": w.x,
                    "y": w.y,
                    "type": w.warp_type,
                    "next_room": w.next_room,
                    "new_x": w.new_x,
                    "new_y": w.new_y,
                }
                for w in room.warps
            ],
            "npcs": [asdict(n) for n in room.npcs],
            "shops": [asdict(s) for s in room.shops],
            "special_objects": [asdict(o) for o in room.special_objects],
        }

    output_path = Path("room_data.json")
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)

    print(f"\nRoom data exported to {output_path}")


if __name__ == "__main__":
    main()

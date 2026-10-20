#pragma once

#include <algorithm>
#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// A skill macro: three skills used one after another from a type-8 key,
	// and a name said aloud as it starts unless it is silent (report 206).
	struct SkillMacro
	{
		std::string name;
		// The wire byte is 1 for silent, 0 to shout the name.
		bool silent = false;
		std::array<int32_t, 3> skills{};
	};

	// The macros by place in the list, which a type-8 key's action names. v83
	// keeps one list for as long as the client runs (CMacroSysMan), so
	// MACRO_SYS_DATA_INIT may come before the character is loaded.
	class SkillMacros
	{
	public:
		static constexpr size_t MAX_MACROS = 5;
		static constexpr size_t MAX_NAME_BYTES = 12;

		// Take a whole new list, cut to MAX_MACROS.
		void replace(std::vector<SkillMacro> list)
		{
			if (list.size() > MAX_MACROS)
				list.resize(MAX_MACROS);

			macros = std::move(list);
		}

		void clear()
		{
			macros.clear();
		}

		size_t size() const
		{
			return macros.size();
		}

		const std::vector<SkillMacro>& get() const
		{
			return macros;
		}

		// The skill in a slot, 0 when there is none (v83 FUN_00631b4d).
		int32_t skill(size_t index, size_t slot) const
		{
			return index < macros.size() && slot < 3 ? macros[index].skills[slot] : 0;
		}

		const std::string& name(size_t index) const
		{
			static const std::string NONE;

			return index < macros.size() ? macros[index].name : NONE;
		}

		// Whether the macro says its name; true past the end (v83 FUN_00631d19).
		bool shouts(size_t index) const
		{
			return index >= macros.size() || !macros[index].silent;
		}

		// Puts a skill in a slot, first adding empty macros that shout up to
		// this one (v83 FUN_00631b26 and FUN_00631aca).
		void set_skill(size_t index, size_t slot, int32_t skill_id)
		{
			if (index >= MAX_MACROS || slot >= 3)
				return;

			grow(index);
			macros[index].skills[slot] = skill_id;
		}

		// Empties a slot of a macro in the list (v83 FUN_00631a99).
		void clear_skill(size_t index, size_t slot)
		{
			if (index < macros.size() && slot < 3)
				macros[index].skills[slot] = 0;
		}

		// Names a macro and says whether it shouts, adding macros up to it like
		// set_skill. Returns whether anything changed, which is when v83 sends
		// the list (FUN_00631d45); a name over MAX_NAME_BYTES changes nothing.
		bool set_name(size_t index, const std::string& name, bool shout)
		{
			if (index >= MAX_MACROS || name.size() > MAX_NAME_BYTES)
				return false;

			grow(index);

			SkillMacro& macro = macros[index];

			if (macro.name == name && macro.silent == !shout)
				return false;

			macro.name = name;
			macro.silent = !shout;

			return true;
		}

	private:
		void grow(size_t index)
		{
			if (macros.size() <= index)
				macros.resize(index + 1);
		}

		std::vector<SkillMacro> macros;
	};
}

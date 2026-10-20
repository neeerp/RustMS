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

	private:
		std::vector<SkillMacro> macros;
	};
}

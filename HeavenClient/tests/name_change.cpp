#include "../Data/NameChange.h"

#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

namespace
{
	ms::name_change::CancelAnswer answer(std::vector<int8_t> bytes)
	{
		ms::InPacket recv(bytes.data(), bytes.size());

		return ms::name_change::read_cancel_answer(recv);
	}
}

int main()
{
	try
	{
		using namespace ms::name_change;

		require(rules_refusal(0) == nullptr, "0 opens the rules");
		require(std::string(rules_refusal(1)).find("already submitted") != std::string::npos, "1 a waiting request");
		require(std::string(rules_refusal(2)).find("banned") != std::string::npos, "2 a ban");
		require(std::string(rules_refusal(3)).find("within a month") != std::string::npos, "3 the month");
		require(std::string(rules_refusal(4)) == UNKNOWN_ERROR, "4, below level 10, has no text of its own");

		require(std::string(check_refusal("Hero", "Zero", false)) == CANNOT_USE, "the name rules first");
		require(std::string(check_refusal("zero", "Zero", true)) == SAME_NAME, "the own name, in any case");
		require(check_refusal("Hero", "Zero", true) == nullptr, "anything else is asked");

		require(std::string(buy_refusal("Zero", "Zero", "Zero")) == SAME_NAME, "never the own name");
		require(std::string(buy_refusal("Hero", "Zero", "")) == CHECK_FIRST, "unchecked");
		require(std::string(buy_refusal("Hero", "Zero", "Hera")) == CHECK_FIRST, "changed since the check");
		require(buy_refusal("Hero", "Zero", "Hero") == nullptr, "the checked name");

		require(check_answer(0) == nullptr, "free");
		require(std::string(check_answer(1)) == IN_USE, "taken");
		require(std::string(check_answer(-1)) == UNKNOWN_ERROR, "negative is an error");

		require(rules_text("a\\r\\nb\\r\\n\\r\\nc") == "a\\nb\\n\\nc", "one break per \\r\\n");
		require(rules_text("THE \xE2\x80\x9CGAME\xE2\x80\x9D") == "THE \"GAME\"", "straight quotes");

		require(answer({ 0 }).kind == CancelAnswer::DONE, "0 cancelled");
		require(answer({ -1 }).kind == CancelAnswer::FAILED, "0xFF failed");

		CancelAnswer unknown = answer({ 1, 0 });
		require(unknown.kind == CancelAnswer::MESSAGE && unknown.message == UNKNOWN_ERROR, "sub 0 is the unknown error");

		CancelAnswer said = answer({ 1, 1, 2, 0, 'n', 'o' });
		require(said.kind == CancelAnswer::MESSAGE && said.message == "no", "otherwise the server's words");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;

		return 1;
	}

	return 0;
}

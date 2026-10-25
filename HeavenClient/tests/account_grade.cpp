#include "../Util/AccountGrade.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using ms::AccountGrade;

		require(!AccountGrade{}.gm() && !AccountGrade{}.admin() && !AccountGrade{}.alerts_admins(), "a player's account");
		require(AccountGrade{ 0x01, 0 }.gm() && !AccountGrade{ 0x01, 0 }.admin(), "a GM's account isn't the admin one");
		require(!AccountGrade{ 0x10, 0 }.gm() && !AccountGrade{ 0x10, 0 }.admin(), "0x10 alone is neither");
		require(AccountGrade{ 0x11, 0 }.gm() && AccountGrade{ 0x11, 0 }.admin(), "0x11 is the admin account, a GM's too");
		require(!AccountGrade{ 0x01, 0x40 }.alerts_admins(), "the sub-grade's 0x40 doesn't tell the server");
		require(AccountGrade{ 0, 0x80 }.alerts_admins() && !AccountGrade{ 0, 0x80 }.gm(), "its 0x80 does, without the GM bit");
	}
	catch (const std::exception& e)
	{
		std::cerr << "account_grade: " << e.what() << '\n';
		return 1;
	}

	std::cout << "account_grade tests passed" << std::endl;
	return 0;
}
